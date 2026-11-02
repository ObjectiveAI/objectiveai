//! One frontend request: a frontend scope on the proxy, its body asked
//! of the caller as a write's content is, its answer relayed.

use std::pin::pin;
use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use diverge_sdk::container_proxy::outside::endpoints::frontend::client::execute as frontend;
use diverge_sdk::shared::containers::frontend::output::Frame;
use diverge_sdk::shared::containers::frontend::request::Request;
use diverge_sdk::wire::server::answers::Answers;

/// Open a `frontend` scope on the proxy with the request's head, feed
/// it the body the caller gives on the ask quoting the request's write
/// id, and put every frame of the answer on the caller's channel as it
/// comes, then the finish.
///
/// The rules:
///
/// 1. Every frame is relayed as it is: the form is the same at both
///    ends (see [`frontend`](diverge_sdk::shared::containers::frontend)).
/// 2. The body is asked for on a channel this end opens, quoting the
///    write id, as a write's content is: each piece goes to the proxy
///    as it comes, and the ask's finish is the body's end. A piece
///    that is the caller's error drops the request.
/// 3. A scope the proxy would not open, or a stream that breaks, is one
///    `Error` in this end's words, the proxy stopped.
/// 4. The proxy's connection ending is the container stopped: the
///    finish alone.
/// 5. The caller abandoning the channel, or its scope ending, stops the
///    proxy's request.
pub(crate) async fn frontend<F: Family>(run: Arc<Run>, channel: u32, request: Request) {
    let Some(ask) = F::write_ask(request.write_id) else {
        run.finish(channel).await;
        return;
    };
    match frontend::execute(&run.proxy, request.head).await {
        Err(error) => run.respond(channel, encoded(&Frame::Error(render::proxy(error)))).await,
        Ok((mut stream, handle)) => {
            // Rule 2.
            let mut body = Answers::open(&run.scope, &ask).await.map(|item| match item {
                Ok(payload) => F::content(&payload),
                Err(_) => Err(render::content_stopped()),
            });
            let feeding = async {
                while let Some(piece) = body.next().await {
                    match piece {
                        Ok(bytes) => {
                            if handle.body(&bytes).await.is_err() {
                                return;
                            }
                        }
                        Err(_) => {
                            let _ = handle.stop().await;
                            return;
                        }
                    }
                }
                let _ = handle.end().await;
            };
            let mut feeding = pin!(feeding);
            let mut fed = false;
            loop {
                tokio::select! {
                    item = stream.next() => match item {
                        // Rule 1.
                        Some(Ok(frame)) => run.respond(channel, encoded(&frame.as_frame())).await,
                        // Rule 4.
                        Some(Err(frontend::ExecuteStreamError::Closed)) | None => break,
                        // Rule 3.
                        Some(Err(error)) => {
                            run.respond(channel, encoded(&Frame::Error(render::proxy(error)))).await;
                            let _ = handle.stop().await;
                            break;
                        }
                    },
                    () = &mut feeding, if !fed => fed = true,
                    // Rule 5.
                    () = run.scope.answer_gone(channel) => {
                        let _ = handle.stop().await;
                        break;
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
