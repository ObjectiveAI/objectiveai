//! `/agent/register`: the agent, forwarded to the agent's `POST
//! /register`.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::agent::register::response;
use futures_util::StreamExt as _;
use reqwest::header::CONTENT_TYPE;

use super::{Upstream, upstream};

/// `/agent/register`. Accepted as many times as the server opens it;
/// once is the agent's server's rule, and its refusal of a second is
/// forwarded like any other.
pub async fn register(State(upstream): State<Arc<Upstream>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, upstream))
        .into_response()
}

/// Serve one registration: the first message is the request,
/// forwarded as the `POST /register` body verbatim — anything else is
/// the clean close with nothing before it; a `2xx` is `Registered`; a
/// non-`2xx`, or a server that cannot be dialed, is `Error`. Then the
/// close.
async fn serve(socket: WebSocket, upstream: Arc<Upstream>) {
    let (sink, mut stream) = socket.split();
    let request = match stream.next().await {
        Some(Ok(Message::Binary(bytes))) => bytes,
        _ => {
            upstream::finish(sink, None).await;
            return;
        }
    };
    let response = upstream
        .http()
        .post(upstream.url("/register"))
        .header(CONTENT_TYPE, "application/json")
        .body(request)
        .send()
        .await;
    let frame = match response {
        Err(error) => response::Frame::Error(upstream::refused(&error)),
        Ok(response) if !response.status().is_success() => {
            response::Frame::Error(upstream::status_error(response).await)
        }
        Ok(_) => response::Frame::Registered,
    };
    upstream::finish(sink, upstream::encoded(&frame)).await;
}
