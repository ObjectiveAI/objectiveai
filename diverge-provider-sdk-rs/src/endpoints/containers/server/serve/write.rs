//! One file into the container, from the caller.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use crate::container_proxy::filesystem::write;
use crate::server::answers::Answers;
use crate::shared::error::Error;

/// The two exchanges of a write, joined: this end opens a channel for
/// the content the caller started under `write_id`, and what arrives
/// on it is streamed into the proxy's `/filesystem/write` for `path`.
/// Then one answer on the caller's channel — written, or why not —
/// and the finish. Content that stopped, from either side, is a
/// write that did not happen; the proxy leaves the destination as it
/// was.
pub(crate) async fn write<F: Family>(run: Arc<Run>, channel: u32, write_id: u32, path: Vec<String>) {
    let Some(ask) = F::write_ask(write_id) else {
        run.finish(channel).await;
        return;
    };
    let content = Answers::open(&run.scope, &ask).await.map(|item| match item {
        Ok(payload) => F::content(&payload),
        Err(_) => Err(render::content_stopped()),
    });
    let request = write::request::Request { path };
    let answer = match write::execute::execute(&run.client, &request, content).await {
        Ok(()) => F::written(),
        Err(error) => failed(error).and_then(|error| F::write_error(&error)),
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// Why the write did not happen, as the caller is told it: the
/// content's own error as it was; the proxy's refusal in its words;
/// a proxy that could not serve the write at all as nothing, the
/// wire's could-not-serve; everything else as the provider's.
fn failed(error: write::execute::ExecuteError<Error>) -> Option<Error> {
    use write::execute::ExecuteError;
    Some(match error {
        ExecuteError::Content(error) => error,
        ExecuteError::Refused(message) => render::refused(&message),
        ExecuteError::Unserved => return None,
        ExecuteError::Open(error) => render::proxy(error),
        ExecuteError::Encode(error) => render::proxy(error),
        ExecuteError::Answer(error) => render::proxy(error),
        ExecuteError::Socket(error) => render::proxy(error),
        ExecuteError::Closed => render::proxy("/filesystem/write ended without a close"),
    })
}
