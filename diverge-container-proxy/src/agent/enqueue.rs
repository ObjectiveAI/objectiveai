//! A message for the agent: the server's enqueue channel, and the
//! whole queue, joined, handed to the loop in flight.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::agent;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::enqueue::response;
use diverge_sdk::shared::error::Error;
use reqwest::StatusCode;
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
/// channel is read, and only the wait is the task's — so a message is
/// in the queue before any channel the server opens after it is read,
/// whatever order their tasks run in. Where it stands there is its
/// `index`'s, not when it came.
pub fn enqueue(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, key: String, index: u64, content: Vec<ContentBlock>) -> impl Future<Output = ()> + Send + 'static {
    let (fate, heard) = oneshot::channel();
    let sent = proxy
        .commands()
        .is_some_and(|commands| commands.send(Cmd::Enqueue(Queued { key, index, content, fate })).is_ok());
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

/// What one `POST /enqueue` came to, every message in the batch
/// alike.
pub enum Outcome {
    /// `204`: the program has the batch, on its own queue, to take at
    /// its next seam as one user turn.
    Delivered,
    /// `409`, the run closed under the batch — the loop's last look
    /// found the program's queue empty — or a `5xx`, a server that
    /// could not be reached, an answer that was not `204`: the batch
    /// is the proxy's again, for the next `/run`.
    Returned,
    /// Another `4xx`: the agent's server refused the batch — content
    /// it will not take — in its own words. Every message's fate is
    /// the error, and none is sent again.
    Refused(Error),
}

/// Hand the batch to the loop in flight, on a task of its own:
/// `POST /enqueue` with every message, in index order, which the
/// program answers at once. The driver reads the outcome when it
/// arrives and never waits on it.
pub fn deliver(proxy: Arc<Proxy>, messages: Vec<Message>) -> oneshot::Receiver<Outcome> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(body) = serde_json::to_vec(&agent::enqueue::request::Request { messages }) else {
            let _ = sender.send(Outcome::Returned);
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
            Ok(response) if response.status() == StatusCode::NO_CONTENT => Outcome::Delivered,
            Ok(response) if response.status() == StatusCode::CONFLICT => Outcome::Returned,
            Ok(response) if response.status().is_client_error() => Outcome::Refused(status_error(response).await),
            Ok(_) | Err(_) => Outcome::Returned,
        };
        let _ = sender.send(outcome);
    });
    receiver
}
