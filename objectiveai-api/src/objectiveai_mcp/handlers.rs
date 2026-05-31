//! Typed delegate functions, one per MCP route. Every forwarding
//! delegate wraps the same primitive: re-build the JSON-RPC envelope,
//! ship it to the CLI via `send_server_request`, await the matching
//! `server_response`, unwrap `result` (or propagate `error`) into the
//! delegate's typed return.

use super::send::send_server_request;
use crate::objectiveai_mcp::context::McpRequestContext;
use axum::http::HeaderMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::server_request;
use objectiveai_sdk::mcp::initialize_result::{
    Implementation, InitializeResult, ResourcesCapability, ServerCapabilities,
    ToolsCapability,
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams,
    ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;

/// How long to wait for a `server_response` over the WS before failing
/// the request as a gateway timeout. Mirrors the SDK conduit endpoint's
/// `REVERSE_CHANNEL_TIMEOUT`.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(30);

/// Common error shape every delegate returns. The route layer renders
/// this into either a JSON-RPC error envelope (under `POST /`) or an
/// HTTP status response (for `DELETE`). Codes follow JSON-RPC
/// conventions; see `routes::mcp_error_to_http` for the mapping.
#[derive(Debug)]
pub struct McpError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn no_session(id: &str) -> Self {
        Self {
            code: -32001,
            message: format!("no reverse channel for response_id {id:?}"),
            data: None,
        }
    }

    pub fn reverse_channel_closed() -> Self {
        Self {
            code: -32002,
            message: "reverse channel closed before request could be sent".into(),
            data: None,
        }
    }

    pub fn reverse_channel_dropped() -> Self {
        Self {
            code: -32002,
            message: "reverse channel dropped before response arrived".into(),
            data: None,
        }
    }

    pub fn reverse_channel_timeout() -> Self {
        Self {
            code: -32003,
            message: "reverse channel timed out waiting for response".into(),
            data: None,
        }
    }

    pub fn empty_response() -> Self {
        Self {
            code: -32004,
            message: "empty response body from reverse channel".into(),
            data: None,
        }
    }

    pub fn missing_result(body: &serde_json::Value) -> Self {
        Self {
            code: -32004,
            message: "reverse channel response missing both `result` and `error`".into(),
            data: Some(body.clone()),
        }
    }

    pub fn parse(message: String) -> Self {
        Self {
            code: -32603,
            message,
            data: None,
        }
    }

    pub fn serialize(message: String) -> Self {
        Self {
            code: -32603,
            message,
            data: None,
        }
    }
}

/// Minimal `initialize` params struct — only `protocolVersion` is
/// load-bearing for the proxy ([`objectiveai-mcp-proxy/src/mcp.rs:246-273`])
/// and the same is true here. `clientInfo` / `capabilities` arrive
/// on the wire and serde drops them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestParams {
    pub protocol_version: String,
}

// ────────────────────────────────────────────────────────────────
// JSON-RPC method delegates (POST /objectiveai-mcp)
// ────────────────────────────────────────────────────────────────

/// Protocol version the API advertises on `initialize`. Pinned —
/// mirrors `objectiveai-mcp-proxy/src/mcp.rs::PROTOCOL_VERSION`.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Canonical `InitializeResult` the API replaces the CLI's body
/// with. The CLI no longer advertises any capabilities — every
/// session presents the same surface: `tools.listChanged=true` +
/// `resources.listChanged=true`, server name `"oai"`. Matches the
/// shape of `objectiveai-mcp-proxy::server_capabilities` exactly,
/// with the server name shortened from `"oaip"` to `"oai"`.
fn canonical_initialize_result() -> InitializeResult {
    InitializeResult {
        protocol_version: PROTOCOL_VERSION.into(),
        capabilities: ServerCapabilities {
            experimental: None,
            logging: None,
            completions: None,
            prompts: None,
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(ResourcesCapability {
                subscribe: None,
                list_changed: Some(true),
            }),
            tasks: None,
        },
        server_info: Implementation {
            name: "oai".into(),
            title: None,
            version: env!("CARGO_PKG_VERSION").into(),
            website_url: None,
            description: None,
            icons: None,
        },
        instructions: None,
        _meta: None,
    }
}

/// `initialize` — forwards to the CLI for the aggregate
/// `Mcp-Session-Id`, then returns the API's canonical result. The
/// CLI's response body is discarded (it returns `body: None` on
/// initialize); only the `Mcp-Session-Id` response header is
/// extracted via [`forward_initialize_session_id`].
///
/// Caller (the route layer) stamps `Mcp-Session-Id` from the
/// returned `String` onto the outbound HTTP response header so the
/// proxy adopts it.
pub async fn handle_initialize(
    ctx: McpRequestContext,
    _params: InitializeRequestParams,
) -> Result<(InitializeResult, String), McpError> {
    let session_id = forward_initialize_session_id(&ctx).await?;
    Ok((canonical_initialize_result(), session_id))
}

/// Forward `initialize` to the CLI conduit and return its
/// `Mcp-Session-Id` response header (the aggregate). Body is
/// ignored — the API replaces it with [`canonical_initialize_result`].
async fn forward_initialize_session_id(
    ctx: &McpRequestContext,
) -> Result<String, McpError> {
    let rc = ctx
        .registry
        .get(&ctx.response_id)
        .ok_or_else(|| McpError::no_session(&ctx.response_id))?
        .clone();

    let request_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "initialize",
        "params": serde_json::Value::Null,
    });
    let request = server_request::Request {
        id: request_id,
        method: "POST".to_string(),
        headers: forward_headers(&ctx.headers),
        body: Some(envelope),
    };

    let rx = send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|_| McpError::reverse_channel_closed())?;

    let response = match tokio::time::timeout(FORWARD_TIMEOUT, rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => return Err(McpError::reverse_channel_dropped()),
        Err(_) => return Err(McpError::reverse_channel_timeout()),
    };

    response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Mcp-Session-Id"))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| McpError::parse(
            "initialize response missing Mcp-Session-Id header".into(),
        ))
}

pub async fn handle_ping(_ctx: McpRequestContext) -> Result<(), McpError> {
    // Local. The route layer 404'd already if the response_id was
    // bogus; we just confirm liveness.
    Ok(())
}

pub async fn handle_tools_list(
    ctx: McpRequestContext,
    params: ListToolsRequest,
) -> Result<ListToolsResult, McpError> {
    let params = serde_json::to_value(params)
        .map_err(|e| McpError::serialize(format!("tools/list params: {e}")))?;
    forward_jsonrpc(&ctx, "tools/list", params).await
}

pub async fn handle_tools_call(
    ctx: McpRequestContext,
    params: CallToolRequestParams,
) -> Result<CallToolResult, McpError> {
    let params = serde_json::to_value(params)
        .map_err(|e| McpError::serialize(format!("tools/call params: {e}")))?;
    forward_jsonrpc(&ctx, "tools/call", params).await
}

pub async fn handle_resources_list(
    ctx: McpRequestContext,
    params: ListResourcesRequest,
) -> Result<ListResourcesResult, McpError> {
    let params = serde_json::to_value(params)
        .map_err(|e| McpError::serialize(format!("resources/list params: {e}")))?;
    forward_jsonrpc(&ctx, "resources/list", params).await
}

pub async fn handle_resources_read(
    ctx: McpRequestContext,
    params: ReadResourceRequestParams,
) -> Result<ReadResourceResult, McpError> {
    let params = serde_json::to_value(params)
        .map_err(|e| McpError::serialize(format!("resources/read params: {e}")))?;
    forward_jsonrpc(&ctx, "resources/read", params).await
}

// ────────────────────────────────────────────────────────────────
// Session lifecycle (DELETE /objectiveai-mcp)
// ────────────────────────────────────────────────────────────────

pub async fn handle_session_terminate(
    ctx: McpRequestContext,
) -> Result<(), McpError> {
    let rc = ctx
        .registry
        .get(&ctx.response_id)
        .ok_or_else(|| McpError::no_session(&ctx.response_id))?
        .clone();

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = server_request::Request {
        id: request_id,
        method: "DELETE".to_string(),
        headers: forward_headers(&ctx.headers),
        body: None,
    };

    let rx = send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|_| McpError::reverse_channel_closed())?;

    match tokio::time::timeout(FORWARD_TIMEOUT, rx).await {
        Ok(Ok(_response)) => Ok(()),
        Ok(Err(_)) => Err(McpError::reverse_channel_dropped()),
        Err(_) => Err(McpError::reverse_channel_timeout()),
    }
}

// ────────────────────────────────────────────────────────────────
// Internal: forward one JSON-RPC method over the WS reverse channel.
// ────────────────────────────────────────────────────────────────

async fn forward_jsonrpc<R: DeserializeOwned>(
    ctx: &McpRequestContext,
    method: &str,
    params: serde_json::Value,
) -> Result<R, McpError> {
    let rc = ctx
        .registry
        .get(&ctx.response_id)
        .ok_or_else(|| McpError::no_session(&ctx.response_id))?
        .clone();

    let request_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params,
    });
    let request = server_request::Request {
        id: request_id,
        method: "POST".to_string(),
        headers: forward_headers(&ctx.headers),
        body: Some(envelope),
    };

    let rx = send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|_| McpError::reverse_channel_closed())?;

    let response = match tokio::time::timeout(FORWARD_TIMEOUT, rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => return Err(McpError::reverse_channel_dropped()),
        Err(_) => return Err(McpError::reverse_channel_timeout()),
    };

    let body = response.body.ok_or_else(McpError::empty_response)?;

    // Upstream returned a JSON-RPC error envelope — preserve its
    // code/message/data verbatim.
    if let Some(err) = body.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603);
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("upstream MCP error")
            .to_string();
        let data = err.get("data").cloned();
        return Err(McpError { code, message, data });
    }

    let result = body
        .get("result")
        .ok_or_else(|| McpError::missing_result(&body))?
        .clone();
    serde_json::from_value(result)
        .map_err(|e| McpError::parse(format!("{method} result: {e}")))
}

/// Copy inbound headers for forwarding, dropping hop-by-hop and
/// transport-routing ones. `Mcp-Session-Id` passes through — that's the
/// standard MCP transport identifier minted by the upstream server.
fn forward_headers(headers: &HeaderMap) -> IndexMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| {
            let name = k.as_str();
            let drop = matches!(
                name.to_ascii_lowercase().as_str(),
                "host"
                    | "content-length"
                    | "connection"
                    | "accept"
                    | "content-type"
                    | "x-objectiveai-response-id"
            );
            if drop {
                return None;
            }
            Some((name.to_string(), v.to_str().ok()?.to_string()))
        })
        .collect()
}
