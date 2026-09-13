//! The agents family's own exchanges: the loop and its queue.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::Run;
use crate::container_proxy::agent;
use crate::shared::containers::{agent_schema, dequeue, enqueue, run_loop};

/// What an agent container's caller opens, past the shared five.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// Run the loop on this prompt.
    AgentRun(String),
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
        Exchange::AgentRun(prompt) => run_loop(run, channel, prompt).await,
        Exchange::AgentSchema => schema(run, channel).await,
        Exchange::Enqueue(prompt) => enqueue(run, channel, prompt).await,
        Exchange::Dequeue => dequeue(run, channel).await,
    }
}

/// The loop's chunks as they come, then the finish; the container's
/// `Error` where there was no loop, or it died, then the finish.
async fn run_loop(run: Arc<Run>, channel: u32, prompt: String) {
    let request = run_loop::request::Request { prompt };
    if let Ok(mut chunks) = agent::run::execute::execute(&run.client, &request).await {
        while let Some(item) = chunks.next().await {
            match item {
                Ok(chunk) => run.respond(channel, encoded(&run_loop::response::Frame::Chunk(chunk))).await,
                Err(agent::run::execute::ExecuteStreamError::Refused(error)) => {
                    run.respond(channel, encoded(&run_loop::response::Frame::Error(error))).await;
                    break;
                }
                Err(_) => break,
            }
        }
    }
    run.finish(channel).await;
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
