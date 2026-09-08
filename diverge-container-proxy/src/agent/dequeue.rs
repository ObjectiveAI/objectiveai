//! `/agent/dequeue`: the queue cleared, forwarded from `POST /dequeue`.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::agent::dequeue::Outcome;
use diverge_provider_sdk::shared::containers::dequeue::response;
use diverge_provider_sdk::shared::error::Error;
use reqwest::header::CONTENT_TYPE;

use super::{Upstream, upstream};

/// `/agent/dequeue`. Accepted as many times as the server opens it,
/// each one clearing.
pub async fn dequeue(State(upstream): State<Arc<Upstream>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, upstream))
        .into_response()
}

/// One `POST /dequeue` with `{}`; its `2xx` body is the outcome, as
/// the frame, and anything else — a non-`2xx`, a server that cannot
/// be dialed, a body that is not an outcome — is `Error`. Then the
/// close.
async fn serve(socket: WebSocket, upstream: Arc<Upstream>) {
    let response = upstream
        .http()
        .post(upstream.url("/dequeue"))
        .header(CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await;
    let frame = match response {
        Err(error) => response::Frame::Error(upstream::refused(&error)),
        Ok(response) if !response.status().is_success() => {
            response::Frame::Error(upstream::status_error(response).await)
        }
        Ok(response) => match response.json::<Outcome>().await {
            Ok(outcome) => response::Frame::from(outcome),
            Err(error) => response::Frame::Error(Error(serde_json::json!({
                "kind": "agent",
                "error": format!("the outcome did not parse: {error}"),
            }))),
        },
    };
    upstream::finish(socket, upstream::encoded(&frame)).await;
}
