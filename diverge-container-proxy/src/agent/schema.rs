//! `/agent/schema`: the agent's schema, forwarded from `GET /schema`.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::shared::containers::agent_schema::response;

use super::{Upstream, upstream};

/// `/agent/schema`. Accepted as many times as the server opens it,
/// each one call.
pub async fn schema(State(upstream): State<Arc<Upstream>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, upstream))
        .into_response()
}

/// One `GET /schema`; its `2xx` body is the schema, verbatim behind
/// the tag — not read here — and anything else is `Error`. Then the
/// close.
async fn serve(socket: WebSocket, upstream: Arc<Upstream>) {
    let frame = match upstream.http().get(upstream.url("/schema")).send().await {
        Err(error) => upstream::encoded(&response::Frame::Error(upstream::refused(&error))),
        Ok(response) if !response.status().is_success() => {
            upstream::encoded(&response::Frame::Error(upstream::status_error(response).await))
        }
        Ok(response) => match response.bytes().await {
            Ok(body) => {
                let mut bytes = Vec::with_capacity(1 + body.len());
                bytes.push(response::AGENT_SCHEMA);
                bytes.extend_from_slice(&body);
                Some(bytes)
            }
            Err(error) => upstream::encoded(&response::Frame::Error(upstream::refused(&error))),
        },
    };
    upstream::finish(socket, frame).await;
}
