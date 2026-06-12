//! Axum sub-router + JSON-RPC dispatch for the API's MCP server.
//!
//! Two route prefixes — one per [`McpKind`] discriminator:
//!
//! - `POST/GET/DELETE /objectiveai`        → [`McpKind::ObjectiveAi`]
//! - `POST/GET/DELETE /{owner}/{name}/{version}/{mcp}` → [`McpKind::Other`]
//!
//! Each MCP-spec method (POST JSON-RPC, GET-SSE notifications, DELETE
//! session-terminate) lives on both prefixes; the path-extracted
//! [`McpKind`] gets threaded into the [`server_request::Request`]
//! envelope so the CLI's per-MCP dispatch table can route directly.
//!
//! Routing key on every request: the `X-OBJECTIVEAI-RESPONSE-ID`
//! header. Missing → 400, unknown → 404. The path is the secondary
//! discriminator (which MCP server this hits within the CLI).

use crate::objectiveai_mcp::{context::McpRequestContext, handlers};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use super::{McpListenerRegistry, ReverseChannelRegistry, handle_get_sse};

const JSON_RPC: &str = "2.0";
const RESPONSE_ID_HEADER: &str = "X-OBJECTIVEAI-RESPONSE-ID";

// ────────────────────────────────────────────────────────────────
// Router builder
// ────────────────────────────────────────────────────────────────

pub fn router(
    reverse_channels: ReverseChannelRegistry,
    listeners: McpListenerRegistry,
    reverse_channel_timeout: std::time::Duration,
) -> axum::Router {
    let state = SharedState {
        reverse_channels,
        listeners,
        reverse_channel_timeout,
    };
    axum::Router::new()
        .route(
            "/objectiveai",
            axum::routing::post(handle_post_objectiveai)
                .get(handle_get_objectiveai)
                .delete(handle_delete_objectiveai),
        )
        .route(
            "/{owner}/{name}/{version}/{mcp}",
            axum::routing::post(handle_post_other)
                .get(handle_get_other)
                .delete(handle_delete_other),
        )
        .with_state(state)
}

#[derive(Clone)]
struct SharedState {
    reverse_channels: ReverseChannelRegistry,
    listeners: McpListenerRegistry,
    reverse_channel_timeout: std::time::Duration,
}

/// Pull the routing key out of the headers, verify a reverse channel
/// exists for it, and return the bare id. Returns the response to send
/// directly on the failure paths.
fn route(state: &SharedState, headers: &HeaderMap) -> Result<String, Response> {
    let response_id = headers
        .get(RESPONSE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let Some(response_id) = response_id else {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("missing {RESPONSE_ID_HEADER} header"),
        )
            .into_response());
    };
    if state.reverse_channels.get(&response_id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("unknown response_id {response_id:?}"),
        )
            .into_response());
    }
    Ok(response_id)
}

fn build_ctx(
    state: &SharedState,
    response_id: String,
    headers: HeaderMap,
) -> McpRequestContext {
    McpRequestContext {
        response_id,
        headers,
        registry: state.reverse_channels.clone(),
        reverse_channel_timeout: state.reverse_channel_timeout,
    }
}

// ────────────────────────────────────────────────────────────────
// Per-prefix handler shims — each parses its McpKind off the path
// (or constructs the unit variant), then delegates to the shared
// helpers below.
// ────────────────────────────────────────────────────────────────

async fn handle_post_objectiveai(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_post(McpKind::ObjectiveAi, state, headers, body).await
}

async fn handle_get_objectiveai(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Response {
    handle_get(McpKind::ObjectiveAi, state, headers).await
}

async fn handle_delete_objectiveai(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Response {
    handle_delete(McpKind::ObjectiveAi, state, headers).await
}

async fn handle_post_other(
    State(state): State<SharedState>,
    Path((owner, name, version, mcp)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_post(
        McpKind::Other { owner, name, version, mcp },
        state,
        headers,
        body,
    )
    .await
}

async fn handle_get_other(
    State(state): State<SharedState>,
    Path((owner, name, version, mcp)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Response {
    handle_get(McpKind::Other { owner, name, version, mcp }, state, headers).await
}

async fn handle_delete_other(
    State(state): State<SharedState>,
    Path((owner, name, version, mcp)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Response {
    handle_delete(McpKind::Other { owner, name, version, mcp }, state, headers).await
}

// ────────────────────────────────────────────────────────────────
// POST — JSON-RPC dispatch
// ────────────────────────────────────────────────────────────────

async fn handle_post(
    mcp_kind: McpKind,
    state: SharedState,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(resp) = require_streamable_http_accept(&headers) {
        return resp;
    }
    let response_id = match route(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let envelope: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return parse_error_response(format!("invalid JSON: {e}")),
    };

    let id = envelope.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = envelope
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    let params = envelope
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let ctx = build_ctx(&state, response_id, headers);

    // `initialize` is special-cased: the API forwards the CLI's
    // verbatim `InitializeResult` AND stamps the upstream's
    // `Mcp-Session-Id` returned by the CLI onto the outbound HTTP
    // response header. Plugin args ride on `X-OBJECTIVEAI-ARGUMENTS`
    // (JSON-serialized declaration-order IndexMap).
    if method == "initialize" {
        let args = parse_args_header(&ctx.headers);
        return match handlers::handle_initialize(ctx, mcp_kind, args).await {
            Ok((result, session_id)) => {
                let value = serde_json::to_value(result)
                    .expect("InitializeResult serializes");
                let mut response = json_rpc_success(id, value);
                if let Ok(v) = axum::http::HeaderValue::from_str(&session_id) {
                    response
                        .headers_mut()
                        .insert("Mcp-Session-Id", v);
                }
                response
            }
            Err(e) => json_rpc_error(id, e),
        };
    }

    let result = match method.as_str() {
        "ping" => dispatch_ping(ctx).await,
        "tools/list" => dispatch_tools_list(ctx, mcp_kind, params).await,
        "tools/call" => dispatch_tools_call(ctx, mcp_kind, params).await,
        "resources/list" => dispatch_resources_list(ctx, mcp_kind, params).await,
        "resources/read" => dispatch_resources_read(ctx, mcp_kind, params).await,
        other => return method_not_found(id, other),
    };

    match result {
        Ok(value) => json_rpc_success(id, value),
        Err(e) => json_rpc_error(id, e),
    }
}

/// Decode the `X-OBJECTIVEAI-ARGUMENTS` header into an ordered
/// argument map. Absent / non-UTF-8 / non-JSON / wrong-shape headers
/// resolve to an empty map; the CLI then spawns the plugin with no
/// `--<arg>` flags. Used only on the `initialize` POST — every other
/// proxy request to the same URL reuses the same header but its args
/// have already been delivered to the CLI at dial time.
fn parse_args_header(headers: &HeaderMap) -> IndexMap<String, Option<String>> {
    headers
        .get("X-OBJECTIVEAI-ARGUMENTS")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

// ────────────────────────────────────────────────────────────────
// Per-method dispatchers — parse params, call delegate, re-erase
// result to serde_json::Value for the success branch.
// ────────────────────────────────────────────────────────────────

async fn dispatch_ping(
    ctx: McpRequestContext,
) -> Result<serde_json::Value, handlers::McpError> {
    handlers::handle_ping(ctx).await?;
    Ok(serde_json::json!({}))
}

async fn dispatch_tools_list(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: serde_json::Value,
) -> Result<serde_json::Value, handlers::McpError> {
    let params = serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("tools/list: {e}")))?;
    let result = handlers::handle_tools_list(ctx, mcp_kind, params).await?;
    Ok(serde_json::to_value(result).expect("ListToolsResult serializes"))
}

async fn dispatch_tools_call(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: serde_json::Value,
) -> Result<serde_json::Value, handlers::McpError> {
    let params = serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("tools/call: {e}")))?;
    let result = handlers::handle_tools_call(ctx, mcp_kind, params).await?;
    Ok(serde_json::to_value(result).expect("CallToolResult serializes"))
}

async fn dispatch_resources_list(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: serde_json::Value,
) -> Result<serde_json::Value, handlers::McpError> {
    let params = serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("resources/list: {e}")))?;
    let result = handlers::handle_resources_list(ctx, mcp_kind, params).await?;
    Ok(serde_json::to_value(result).expect("ListResourcesResult serializes"))
}

async fn dispatch_resources_read(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: serde_json::Value,
) -> Result<serde_json::Value, handlers::McpError> {
    let params = serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("resources/read: {e}")))?;
    let result = handlers::handle_resources_read(ctx, mcp_kind, params).await?;
    Ok(serde_json::to_value(result).expect("ReadResourceResult serializes"))
}

// ────────────────────────────────────────────────────────────────
// GET — SSE notifications stream (per-MCP)
// ────────────────────────────────────────────────────────────────

async fn handle_get(
    mcp_kind: McpKind,
    state: SharedState,
    headers: HeaderMap,
) -> Response {
    let response_id = match route(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    handle_get_sse(response_id, mcp_kind, state.listeners.clone(), headers).await
}

// ────────────────────────────────────────────────────────────────
// DELETE — session terminate (per-MCP)
// ────────────────────────────────────────────────────────────────

async fn handle_delete(
    mcp_kind: McpKind,
    state: SharedState,
    headers: HeaderMap,
) -> Response {
    let response_id = match route(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let ctx = build_ctx(&state, response_id, headers);
    match handlers::handle_session_terminate(ctx, mcp_kind).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => mcp_error_to_http(e),
    }
}

// ────────────────────────────────────────────────────────────────
// Envelope + error helpers
// ────────────────────────────────────────────────────────────────

fn json_rpc_success(id: serde_json::Value, result: serde_json::Value) -> Response {
    axum::Json(serde_json::json!({
        "jsonrpc": JSON_RPC,
        "id": id,
        "result": result,
    }))
    .into_response()
}

fn json_rpc_error(id: serde_json::Value, e: handlers::McpError) -> Response {
    let mut err = serde_json::json!({ "code": e.code, "message": e.message });
    if let Some(data) = e.data {
        err["data"] = data;
    }
    axum::Json(serde_json::json!({
        "jsonrpc": JSON_RPC,
        "id": id,
        "error": err,
    }))
    .into_response()
}

fn method_not_found(id: serde_json::Value, method: &str) -> Response {
    json_rpc_error(
        id,
        handlers::McpError {
            code: -32601,
            message: format!("method not found: {method}"),
            data: None,
        },
    )
}

fn invalid_params(message: String) -> handlers::McpError {
    handlers::McpError {
        code: -32602,
        message,
        data: None,
    }
}

fn parse_error_response(message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({
            "jsonrpc": JSON_RPC,
            "id": serde_json::Value::Null,
            "error": { "code": -32700, "message": message },
        })),
    )
        .into_response()
}

/// MCP Streamable-HTTP transport requires `Accept` to list both
/// `application/json` AND `text/event-stream` (or a wildcard); mirrors
/// `objectiveai-mcp-proxy/src/mcp.rs::require_streamable_http_accept`.
fn require_streamable_http_accept(headers: &HeaderMap) -> Result<(), Response> {
    let raw = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut json = false;
    let mut sse = false;
    let mut wildcard = false;
    for part in raw.split(',') {
        let media = part
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match media.as_str() {
            "application/json" => json = true,
            "text/event-stream" => sse = true,
            "*/*" | "application/*" | "text/*" => wildcard = true,
            _ => {}
        }
    }

    if (json && sse) || wildcard {
        Ok(())
    } else {
        Err((
            StatusCode::NOT_ACCEPTABLE,
            "Accept header must list both application/json and text/event-stream",
        )
            .into_response())
    }
}

fn mcp_error_to_http(e: handlers::McpError) -> Response {
    let status = match e.code {
        -32601 => StatusCode::NOT_FOUND,
        -32602 => StatusCode::BAD_REQUEST,
        -32001 => StatusCode::NOT_FOUND,
        -32002 => StatusCode::SERVICE_UNAVAILABLE,
        -32003 => StatusCode::GATEWAY_TIMEOUT,
        -32004 => StatusCode::BAD_GATEWAY,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, e.message).into_response()
}
