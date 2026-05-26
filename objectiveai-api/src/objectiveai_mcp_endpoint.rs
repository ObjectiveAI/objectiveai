//! Streamable-HTTP MCP endpoint the API hosts on
//! `/objectiveai-mcp/{session_id}`. Acts as a pure HTTP→WS bridge:
//! every HTTP request the proxy makes (POST/GET/DELETE) is
//! forwarded verbatim — method, headers, body — over the matching
//! session's [`crate::streaming_ws::ReverseChannel`] as a
//! [`server_request::Request`](objectiveai_sdk::client_objectiveai_mcp::server_request::Request).
//! The calling client's `McpHandler` (typically the CLI's
//! `ConduitMcpHandler`, which spawns objectiveai-mcp and pipes
//! requests through) replies with a
//! [`server_response::Response`](objectiveai_sdk::client_objectiveai_mcp::server_response::Response);
//! the API translates that into the HTTP response the proxy sees.
//!
//! No JSON-RPC dispatch lives here — the API doesn't know about
//! `initialize` / `ping` / `tools/list` / `tools/call`. It just
//! forwards bytes and bytes back.

use std::time::Duration;

use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::server_request;

use crate::streaming_ws::{ReverseChannelRegistry, send_server_request};

/// How long to wait for a `server_response` before failing with
/// `504 Gateway Timeout`. Matches the proxy's default call timeout.
const REVERSE_CHANNEL_TIMEOUT: Duration = Duration::from_secs(30);

/// Forward one HTTP request the proxy made to the calling client's
/// reverse-attach handler and translate the reply back into an
/// HTTP response. Mounted as `POST` / `GET` / `DELETE` on the same
/// route; all three converge here.
pub async fn handle_request(
    session_id: String,
    method: Method,
    registry: ReverseChannelRegistry,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let rc = match registry.get(&session_id) {
        Some(rc) => rc.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("no reverse channel for session_id {session_id:?}"),
            )
                .into_response();
        }
    };

    // Verbatim copy of the proxy's HTTP headers (modulo
    // unrepresentable values).
    let forward_headers: IndexMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_string(), v.to_str().ok()?.to_string())))
        .collect();

    // Body as JSON Value for POSTs; None for GET / DELETE / empty.
    let body_value: Option<serde_json::Value> = if body.is_empty() {
        None
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => Some(v),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("body is not valid JSON: {e}"),
                )
                    .into_response();
            }
        }
    };

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = server_request::Request {
        id: request_id.clone(),
        method: method.as_str().to_string(),
        headers: forward_headers,
        body: body_value,
    };

    let rx = match send_server_request(&rc.sink, &rc.pending, request).await {
        Ok(rx) => {
            rx
        }
        Err(()) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "reverse channel closed before request could be sent",
            )
                .into_response();
        }
    };

    let server_resp = match tokio::time::timeout(REVERSE_CHANNEL_TIMEOUT, rx).await {
        Ok(Ok(r)) => {
            r
        }
        Ok(Err(_)) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "reverse channel dropped before response arrived",
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::GATEWAY_TIMEOUT,
                "reverse channel timed out waiting for response",
            )
                .into_response();
        }
    };

    // Translate the server_response::Response into an HTTP response.
    // The URL path `session_id` is the WS reverse-channel routing
    // key only — it is NOT the MCP session id. Each agent's MCP
    // connection has its own remote-minted `Mcp-Session-Id` that
    // flows transparently through the conduit's response back to
    // the proxy. We pass the header through verbatim so the proxy's
    // SDK client adopts the right id on `initialize` and stamps it
    // on subsequent requests.
    let status = StatusCode::from_u16(server_resp.status)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = axum::response::Response::builder().status(status);
    let mut has_content_type = false;
    for (k, v) in &server_resp.headers {
        if let Ok(value) = HeaderValue::from_str(v) {
            if k.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            builder = builder.header(k, value);
        }
    }
    // Default Content-Type to application/json when a JSON body is
    // present and the handler didn't supply one of its own.
    if !has_content_type && server_resp.body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }

    let body_bytes: Vec<u8> = match server_resp.body {
        Some(v) => serde_json::to_vec(&v).unwrap_or_default(),
        None => Vec::new(),
    };

    let resp = builder
        .body(axum::body::Body::from(body_bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    resp
}
