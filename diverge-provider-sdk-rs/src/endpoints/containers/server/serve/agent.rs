//! The agents family's own exchanges: the queue's two verbs, channels
//! on the begin scope.

use std::sync::Arc;

use super::super::begin::Begin;
use super::super::encoded::encoded;
use super::super::run::Run;

/// What an agent container's caller opens, past the shared six.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// A message for the agent.
    Enqueue(String),
    /// Empty the queue.
    Dequeue,
}

/// Serve one, to the end: the proxy's one answer on the caller's
/// channel, then the finish — or the finish alone where the proxy
/// could not serve it. What the agent says in reply is not here: it
/// rides the main stream, relayed by `relay::chunks`.
pub(crate) async fn serve(run: Arc<Run>, channel: u32, exchange: Exchange) {
    let Begin::Agents(begin) = &run.begin else {
        // A tool container has no queue; nothing classifies into this
        // on one.
        run.finish(channel).await;
        return;
    };
    let answer = match exchange {
        Exchange::Enqueue(prompt) => begin.enqueue(prompt).await.ok().and_then(|frame| encoded(&frame)),
        Exchange::Dequeue => begin.dequeue().await.ok().and_then(|frame| encoded(&frame)),
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
