//! `/tool/read-resource`: one resource of the container's server, read.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::shared::mcp::read_resource::{request, response};
use futures_util::SinkExt as _;
use rmcp::ErrorData;

use super::{Tool, outcome};
use crate::agent;
use crate::ws;

/// `/tool/read-resource`. Accepted as many times as the server opens it,
/// each one exchange.
pub async fn read_resource(State(tool): State<Arc<Tool>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| serve(socket, tool))
        .into_response()
}

/// The first binary message is the params; the one answer is the
/// result or the error, then the close. Params that will not parse
/// are the error too — the caller asked and is owed an answer in the
/// vocabulary it asked in.
async fn serve(mut socket: WebSocket, tool: Arc<Tool>) {
    let Some(bytes) = ws::binary(&mut socket).await else {
        // Closed before the params: nothing was asked.
        let _ = socket.close().await;
        return;
    };
    let params = request::Request::decode(&bytes)
        .map_err(|error| ErrorData::invalid_params(error.to_string(), None));
    let frame = match params {
        Err(error) => response::Frame::Error(error),
        Ok(request) => match tool.client().await {
            Err(error) => response::Frame::Error(error),
            Ok(client) => match outcome(client.read_resource(request.0).await) {
                Ok(result) => response::Frame::Result(result),
                Err(error) => response::Frame::Error(error),
            },
        },
    };
    agent::finish(socket, agent::encoded(&frame)).await;
}
