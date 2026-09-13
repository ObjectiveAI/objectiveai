//! The agents family's own exchanges: the schema and the queue,
//! channels on the begin scope.

use std::sync::Arc;

use super::super::begin::Begin;
use super::super::encoded::encoded;
use super::super::run::Run;
use crate::endpoints::containers::client::UnaryError;
use crate::shared::containers::agent_schema;

/// What an agent container's caller opens, past the shared five.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// The schema of the agent value.
    AgentSchema,
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
        // A tool container has no agent; nothing classifies into
        // this on one.
        run.finish(channel).await;
        return;
    };
    let answer = match exchange {
        Exchange::AgentSchema => match begin.agent_schema().await {
            Ok(schema) => encoded(&agent_schema::response::Frame::AgentSchema(schema)),
            Err(UnaryError::Refused(error)) => encoded(&agent_schema::response::Frame::Error(error)),
            Err(_) => None,
        },
        Exchange::Enqueue(prompt) => begin.enqueue(prompt).await.ok().and_then(|frame| encoded(&frame)),
        Exchange::Dequeue => begin.dequeue().await.ok().and_then(|frame| encoded(&frame)),
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
