//! The loop, run: `POST /run`, its chunks relayed, the secrets fed
//! before it and collected after it, and the proxy's word before and
//! after that a loop began and ended.

use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::container_proxy::inside::agent::run::request::{Message, Request};
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response::{ACTIVE, CHUNK};
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use eventsource_stream::Eventsource as _;
use futures_util::StreamExt as _;
use reqwest::header::CONTENT_TYPE;
use tokio::sync::oneshot;

use crate::program::{self, refused, status_error};
use crate::proxy::Proxy;

/// Start one loop on `messages`, on a task of its own: what comes back
/// is the `2xx` response whose body is the loop, or the error — a
/// non-`2xx` in the agent's server's own words, or a server that
/// could not be reached. The driver reads it when it arrives and
/// never waits on it.
pub fn start(proxy: Arc<Proxy>, messages: Vec<Message>) -> oneshot::Receiver<Result<reqwest::Response, Error>> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let body = match serde_json::to_vec(&Request { messages }) {
            Ok(body) => body,
            Err(error) => {
                let _ = sender.send(Err(Error(serde_json::json!({
                    "kind": "agent",
                    "error": format!("the messages did not serialize: {error}"),
                }))));
                return;
            }
        };
        let response = proxy
            .upstream
            .http()
            .post(proxy.upstream.url("/run"))
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await;
        let started = match response {
            Err(error) => Err(refused(&error)),
            Ok(response) if !response.status().is_success() => Err(status_error(response).await),
            Ok(response) => Ok(response),
        };
        let _ = sender.send(started);
    });
    receiver
}

/// Feed `values` to the program, on a task of its own: `POST
/// /secrets`, held until the program has applied them. What comes
/// back is the word that it has, or the program's refusal in its own
/// words. The driver reads it when it arrives — and opens the gate on
/// it — and never waits on it.
pub fn feed(proxy: Arc<Proxy>, values: Values) -> oneshot::Receiver<Result<(), Error>> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let _ = sender.send(program::feed(&proxy.upstream, values).await);
    });
    receiver
}

/// Relay the loop, on a task of its own: first the proxy's own word
/// that the loop began, `Active`, one byte; then every event's data
/// out on the begin scope's main stream as one `Chunk`, its JSON as
/// the agent's server wrote it, nothing set under its `_meta` — the
/// messages' user parts first, then whatever the agent says — until
/// the stream ends — cleanly, or by dying — which is the loop over;
/// then the rotating secrets are collected from the program — not
/// asked for at all when it declared none — and the outcome handed
/// to the driver, which writes `Inactive` with the values, or the
/// `Error` that ends the run, in the one step that closes the gate,
/// so no `Fed` is ever ordered before the close. The word that the
/// loop began is the proxy's alone: the agent's server produces
/// chunks, every one under the chunk's tag, so nothing it writes is
/// read as it. Nothing is said about HOW the loop ended: a loop that
/// failed already said so in a fatal notification chunk.
pub fn relay(proxy: Arc<Proxy>, response: reqwest::Response, scope: Arc<ScopeHandle>) -> oneshot::Receiver<Result<Values, Error>> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        scope.send_response(&[ACTIVE]).await;
        let mut events = pin!(response.bytes_stream().eventsource());
        while let Some(Ok(event)) = events.next().await {
            // The chunk's tag, then its JSON as the agent's server
            // wrote it.
            let chunk = event.data.as_bytes();
            let mut bytes = Vec::with_capacity(1 + chunk.len());
            bytes.push(CHUNK);
            bytes.extend_from_slice(chunk);
            scope.send_response(&bytes).await;
        }
        let names = proxy.rotating();
        let collected = if names.is_empty() {
            Ok(Values::new())
        } else {
            program::collect(&proxy.upstream, &names).await
        };
        let _ = sender.send(collected);
    });
    receiver
}
