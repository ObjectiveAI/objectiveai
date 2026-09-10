//! `/tool/call-tool`: one tool of the container's server, called.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::shared::mcp::call_tool::{request, response};
use futures_util::StreamExt as _;
use rmcp::ErrorData;

use super::{Tool, outcome};
use crate::agent;

/// `/tool/call-tool`. Accepted as many times as the server opens it,
/// each one exchange.
pub async fn call_tool(State(tool): State<Arc<Tool>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, tool))
        .into_response()
}

/// The first message is the params; the one answer is the result or
/// the error, then the close. Params that are not a binary message,
/// or will not parse, are the error too — the caller asked and is
/// owed an answer in the vocabulary it asked in.
async fn serve(mut socket: WebSocket, tool: Arc<Tool>) {
    let params = match socket.next().await {
        Some(Ok(Message::Binary(bytes))) => request::Request::decode(&bytes)
            .map_err(|error| ErrorData::invalid_params(error.to_string(), None)),
        _ => Err(ErrorData::invalid_params("the params were not a binary message", None)),
    };
    let frame = match params {
        Err(error) => response::Frame::Error(error),
        Ok(request) => match tool.client().await {
            Err(error) => response::Frame::Error(error),
            Ok(client) => match outcome(client.call_tool(request.0).await) {
                Ok(result) => response::Frame::Result(result),
                Err(error) => response::Frame::Error(error),
            },
        },
    };
    agent::finish(socket, agent::encoded(&frame)).await;
}
