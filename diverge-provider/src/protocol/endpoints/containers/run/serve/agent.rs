//! The agents family's own exchange: a batch of messages for the
//! agent, a channel on the begin scope.

use std::sync::Arc;

use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::error::Error;

use super::super::begin::Begin;
use super::super::encoded::encoded;
use super::super::run::Run;

/// What an agent container's caller opens, past the shared six.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// A batch of messages for the agent, in order, carried whole and
    /// not read.
    Enqueue(enqueue::request::Request),
}

/// Serve one, to the end: the proxy's one answer on the caller's
/// channel, then the finish. Where the exchange with the proxy failed
/// — the proxy finished without an answer, the connection to it ended,
/// its answer did not parse — the answer is the `Error` saying so,
/// kind `proxy`, so the caller learns why and not only that. What the
/// agent says in reply is not here: it rides the main stream, relayed
/// by `relay::chunks`.
pub(crate) async fn serve(run: Arc<Run>, channel: u32, exchange: Exchange) {
    let Begin::Agents(begin) = &run.begin else {
        // A tool container has no queue; nothing classifies into this
        // on one.
        run.finish(channel).await;
        return;
    };
    let answer = match exchange {
        Exchange::Enqueue(request) => {
            let frame = begin.enqueue(request).await.unwrap_or_else(|error| enqueue::response::Frame::Error(failed(&error)));
            encoded(&frame)
        }
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// The exchange with the proxy that failed, as the caller is told it.
fn failed(error: &dyn std::fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "proxy",
        "error": error.to_string(),
    }))
}
