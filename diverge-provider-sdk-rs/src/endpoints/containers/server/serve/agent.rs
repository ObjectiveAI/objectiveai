//! The agents family's own exchanges: the schema and the queue.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::run::Run;
use crate::container_proxy::agent;
use crate::shared::containers::{agent_schema, dequeue, enqueue};

/// What an agent container's caller opens, past the shared five.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// The schema of the agent value.
    AgentSchema,
    /// A message for the loop's queue.
    Enqueue(String),
    /// Empty the queue.
    Dequeue,
}

/// Serve one, to the end.
pub(crate) async fn serve(run: Arc<Run>, channel: u32, exchange: Exchange) {
    match exchange {
        Exchange::AgentSchema => schema(run, channel).await,
        Exchange::Enqueue(prompt) => enqueue(run, channel, prompt).await,
        Exchange::Dequeue => dequeue(run, channel).await,
    }
}

/// The schema, or the container's `Error`, then the finish.
async fn schema(run: Arc<Run>, channel: u32) {
    let answer = match agent::schema::execute::execute(&run.client).await {
        Ok(schema) => encoded(&agent_schema::response::Frame::AgentSchema(schema)),
        Err(agent::schema::execute::ExecuteError::Refused(error)) => {
            encoded(&agent_schema::response::Frame::Error(error))
        }
        Err(_) => None,
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// The message's fate, however long it takes, then the finish.
///
/// Relayed to the old proxy's `/agent/enqueue` as it stands. Starting
/// a loop on a message when none runs, and carrying the agent's
/// chunks to the scope's main stream, wait on the proxy's rewrite.
async fn enqueue(run: Arc<Run>, channel: u32, prompt: String) {
    let request = enqueue::request::Request { prompt };
    let answer = agent::enqueue::execute::execute(&run.client, &request)
        .await
        .ok()
        .and_then(|frame: enqueue::response::Frame| encoded(&frame));
    run.respond(channel, answer).await;
    run.finish(channel).await;
}

/// Whether the queue held anything, then the finish.
async fn dequeue(run: Arc<Run>, channel: u32) {
    let answer = agent::dequeue::execute::execute(&run.client)
        .await
        .ok()
        .and_then(|frame: dequeue::response::Frame| encoded(&frame));
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
