//! The loop, run: `POST /run`, and its chunks relayed.

use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::container_proxy::inside::agent::run::request::{Message, Request};
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response::CHUNK;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use eventsource_stream::Eventsource as _;
use futures_util::StreamExt as _;
use reqwest::header::CONTENT_TYPE;
use tokio::sync::oneshot;

use crate::program::{refused, status_error};
use crate::proxy::Proxy;
use crate::stamp::Stamp;

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

/// Relay the loop, on a task of its own: every event's data goes out
/// on the begin scope's main stream as one `Chunk`, its JSON as the
/// agent's server wrote it with the container's image under its
/// `_meta` — the messages' user parts first, then whatever the agent
/// says — until the stream ends — cleanly,
/// or by dying — which is the loop over. The receiver hears the end
/// as its sender dropping. Nothing is said on the stream about how
/// the loop ended: an error there would end the scope, and the
/// stream has no marker between one loop and the next by design.
pub fn relay(response: reqwest::Response, scope: Arc<ScopeHandle>, stamp: Stamp) -> oneshot::Receiver<()> {
    let (sender, receiver) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let mut events = pin!(response.bytes_stream().eventsource());
        while let Some(Ok(event)) = events.next().await {
            // The chunk's tag, then its JSON as the agent's server
            // wrote it, the image under its `_meta`.
            let chunk = stamp.chunk(&event.data);
            let mut bytes = Vec::with_capacity(1 + chunk.len());
            bytes.push(CHUNK);
            bytes.extend_from_slice(&chunk);
            scope.send_response(&bytes).await;
        }
        drop(sender);
    });
    receiver
}
