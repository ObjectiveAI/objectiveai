//! A message for the agent: the server's enqueue channel, and the
//! offer of the whole queue, joined, to the loop in flight.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::agent;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::enqueue::response;
use diverge_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;
use rmcp::model::ContentBlock;
use tokio::sync::oneshot;

use super::{Cmd, Queued};
use crate::encode::encoded;
use crate::program::status_error;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Serve one enqueue channel: the message goes to the driver, and the
/// channel is answered with its fate whenever that is known — one
/// frame, then the finish. Nothing times it out. A driver that is
/// gone, or one that never was, is the finish with nothing before it.
///
/// Not an `async fn`: the message goes to the driver HERE, as the
/// channel is read, and only the wait is the task's — so the driver
/// hears the server's enqueues and dequeues in the order the server
/// opened them, whatever order their tasks run in, and the queue is
/// in enqueue order exactly.
pub fn enqueue(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, key: String, content: Vec<ContentBlock>) -> impl Future<Output = ()> + Send + 'static {
    let (fate, heard) = oneshot::channel();
    let sent = proxy
        .commands()
        .is_some_and(|commands| commands.send(Cmd::Enqueue(Queued { key, content, fate })).is_ok());
    async move {
        if !sent {
            reply(&scope, channel, None).await;
            return;
        }
        let payload = match heard.await {
            Ok(fate) => encoded(&response::Frame::from(fate)),
            Err(_) => None,
        };
        reply(&scope, channel, payload).await;
    }
}

/// What one `POST /enqueue` came to: the offered batch's fate, every
/// message in it alike.
pub enum Outcome {
    /// The loop took the batch, as one user turn.
    Delivered,
    /// The agent's server gave it back on a `/dequeue`: the batch is
    /// the proxy's again, to offer again without what was withdrawn.
    Dequeued,
    /// The loop ended under it: the batch is the proxy's again.
    Missed,
    /// The agent's server refused the batch — a `4xx`, content it
    /// will not take — in its own words: every message's fate is the
    /// error, and none is offered again.
    Refused(Error),
    /// A `5xx`, a server that could not be reached, or a fate that
    /// did not parse: the loop in flight did not take it, and the
    /// batch is the proxy's again.
    Failed,
}

/// Offer the batch to the loop in flight, on a task of its own:
/// `POST /enqueue` with every message, in enqueue order, held until
/// the agent's server says what became of its one slot. The driver
/// reads the outcome when it arrives and never waits on it.
pub fn deliver(proxy: Arc<Proxy>, messages: Vec<Message>) -> oneshot::Receiver<Outcome> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(body) = serde_json::to_vec(&agent::enqueue::request::Request { messages }) else {
            let _ = sender.send(Outcome::Failed);
            return;
        };
        let response = proxy
            .upstream
            .http()
            .post(proxy.upstream.url("/enqueue"))
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await;
        let outcome = match response {
            Ok(response) if response.status().is_success() => match response.json::<agent::enqueue::Fate>().await {
                Ok(agent::enqueue::Fate::Delivered) => Outcome::Delivered,
                Ok(agent::enqueue::Fate::Dequeued) => Outcome::Dequeued,
                Ok(agent::enqueue::Fate::Missed) => Outcome::Missed,
                Err(_) => Outcome::Failed,
            },
            Ok(response) if response.status().is_client_error() => Outcome::Refused(status_error(response).await),
            Ok(_) | Err(_) => Outcome::Failed,
        };
        let _ = sender.send(outcome);
    });
    receiver
}
