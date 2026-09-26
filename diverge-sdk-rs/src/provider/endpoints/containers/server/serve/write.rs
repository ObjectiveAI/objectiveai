//! One file, written into the container: the caller's content, streamed
//! into a scope on the proxy.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy::outside::filesystem::write::client::execute as write;
use crate::wire::server::answers::Answers;
use crate::shared::error::Error;

/// Ask the caller for the write's content on a channel this end
/// opens, quoting its write id; stream it into a `filesystem::write`
/// scope on the proxy; and answer the caller's channel with the
/// proxy's answer — written, or the error — then the finish. A proxy
/// that could not serve the write at all is relayed as the finish
/// alone.
pub(crate) async fn write<F: Family>(run: Arc<Run>, channel: u32, write_id: u32, path: Vec<String>) {
    let Some(ask) = F::write_ask(write_id) else {
        run.finish(channel).await;
        return;
    };
    let content = Answers::open(&run.scope, &ask).await.map(|item| match item {
        Ok(payload) => F::content(&payload),
        Err(_) => Err(render::content_stopped()),
    });
    let answer = match write::execute(&run.proxy, path, content).await {
        Ok(()) => F::written(),
        Err(error) => failed(error).and_then(|error| F::write_error(&error)),
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// What a write that did not land is answered with: the proxy's own
/// words, the caller's own error, this end's failure to reach the
/// proxy — or nothing, for a proxy that could not serve it at all.
pub(super) fn failed(error: write::ExecuteError) -> Option<Error> {
    use write::ExecuteError;
    Some(match error {
        ExecuteError::Refused(error) | ExecuteError::Content(error) => error,
        ExecuteError::Unanswered => return None,
        other => render::proxy(other),
    })
}
