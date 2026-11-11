//! One command, run in the container: an exec scope on the proxy, its
//! stream relayed.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::render;
use super::super::run::Run;
use diverge_sdk::container_proxy::outside::endpoints::exec::client::execute as exec;
use diverge_sdk::shared::containers::exec::request::Request;
use diverge_sdk::shared::containers::exec::response::Frame;

/// Open an `exec` scope on the proxy and put every frame of the
/// command's stream on the caller's channel as it comes, then the
/// finish.
///
/// The rules:
///
/// 1. Every frame is relayed as it is: the form is the same at both
///    ends (see [`exec`](diverge_sdk::shared::containers::exec)).
/// 2. A scope the proxy would not open, or a stream that breaks, is one
///    `Error` in this end's words, the proxy stopped.
/// 3. The proxy's connection ending is the container stopped: the
///    finish alone, as the command is gone with it.
/// 4. The caller abandoning the channel, or its scope ending, stops the
///    proxy's command, which kills it.
pub(crate) async fn exec(run: Arc<Run>, channel: u32, command: Request) {
    match exec::execute(&run.proxy, command).await {
        Err(error) => run.respond(channel, encoded(&Frame::Error(render::proxy(error)))).await,
        Ok((mut stream, handle)) => loop {
            tokio::select! {
                item = stream.next() => match item {
                    // Rule 1.
                    Some(Ok(frame)) => run.respond(channel, encoded(&frame.as_frame())).await,
                    // Rule 3.
                    Some(Err(exec::ExecuteStreamError::Closed)) | None => break,
                    // Rule 2.
                    Some(Err(error)) => {
                        run.respond(channel, encoded(&Frame::Error(render::proxy(error)))).await;
                        let _ = handle.cancel().await;
                        break;
                    }
                },
                // Rule 4.
                () = run.scope.answer_gone(channel) => {
                    let _ = handle.cancel().await;
                    break;
                }
            }
        },
    }
    run.finish(channel).await;
}
