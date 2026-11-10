//! The queue, cleared: the server's dequeue channel, and the offered
//! batch withdrawn from the agent's server.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::agent::dequeue::{Outcome, request};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::dequeue::response;
use diverge_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;
use tokio::sync::oneshot;

use super::Cmd;
use crate::program::{refused, status_error};
use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// Serve one dequeue channel: the key goes to the driver, and the
/// channel is answered with what the driver says — one frame, then
/// the finish. The driver gives back every message still waiting
/// under the key at once, each answered `Dequeued` on its own
/// channel; when the batch offered to the loop holds the key, it
/// withdraws that batch and answers only once the batch's outcome is
/// known, so the answer is exact: `Dequeued` when anything under the
/// key was withdrawn, `Empty` when nothing was, `Error` when the
/// agent's server could not be asked. A driver that is gone, or one
/// that never was, is the finish with nothing before it.
///
/// Not an `async fn`, for [`enqueue`](super::enqueue())'s reason: the
/// key goes to the driver as the channel is read, in the server's
/// order, and only the wait is the task's.
pub fn dequeue(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, key: String) -> impl Future<Output = ()> + Send + 'static {
    let (reply_sender, heard) = oneshot::channel();
    let sent = proxy
        .commands()
        .is_some_and(|commands| commands.send(Cmd::Dequeue { key, reply: reply_sender }).is_ok());
    async move {
        if !sent {
            reply(&scope, channel, None).await;
            return;
        }
        let payload = match heard.await {
            Ok(replied) => encoded(&response::Frame::from(replied)),
            Err(_) => None,
        };
        reply(&scope, channel, payload).await;
    }
}

/// Withdraw the offered batch, on a task of its own: `POST /dequeue`,
/// `{}`, which empties the agent's server's one slot. What comes back
/// is what the agent's server found — the slot's fate is the held
/// `/enqueue`'s to say, and the driver reads it there, but an `empty`
/// tells the driver to ask again — or the error, in the server's own
/// words. The driver reads it when it arrives — and offers nothing
/// until it has, so no withdrawal can land on a batch offered after
/// it — and never waits on it.
pub fn withdraw(proxy: Arc<Proxy>) -> oneshot::Receiver<Result<Outcome, Error>> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let Ok(body) = serde_json::to_vec(&request::Request {}) else {
            let _ = sender.send(Err(Error(serde_json::json!({
                "kind": "agent",
                "error": "the withdrawal did not serialize",
            }))));
            return;
        };
        let response = proxy
            .upstream
            .http()
            .post(proxy.upstream.url("/dequeue"))
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await;
        let withdrawn = match response {
            Err(error) => Err(refused(&error)),
            Ok(response) if !response.status().is_success() => Err(status_error(response).await),
            Ok(response) => response.json::<Outcome>().await.map_err(|error| {
                Error(serde_json::json!({
                    "kind": "agent",
                    "error": format!("the outcome did not parse: {error}"),
                }))
            }),
        };
        let _ = sender.send(withdrawn);
    });
    receiver
}
