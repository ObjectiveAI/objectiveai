//! Typed delegate functions, one per MCP route. Every forwarding
//! delegate wraps the same primitive: build a typed
//! `server_request::Payload`, ship it to the CLI via
//! `send_server_request`, await the matching `server_response`,
//! pattern-match the expected `server_response::Payload` variant
//! (`JsonRpcResult::Ok` → typed result, `Err` → propagate as
//! [`McpError`]).

use super::send::send_server_request;
use crate::objectiveai_mcp::context::McpRequestContext;
use axum::http::HeaderMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
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
use serde::{Deserialize, Serialize};
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

    pub fn variant_mismatch(expected: &str, got: &server_response::Payload) -> Self {
        Self {
            code: -32603,
            message: format!(
                "reverse channel returned wrong payload variant: expected {expected}, got {}",
                payload_variant_name(got),
            ),
            data: None,
        }
    }
}

fn payload_variant_name(p: &server_response::Payload) -> &'static str {
    use server_response::Payload as P;
    match p {
        P::Initialize { .. } => "initialize",
        P::ToolsList(_) => "tools_list",
        P::ToolsCall(_) => "tools_call",
        P::ResourcesList(_) => "resources_list",
        P::ResourcesRead(_) => "resources_read",
        P::SessionTerminate => "session_terminate",
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

/// `initialize` — forward to the CLI, take its returned
/// `mcp_session_id`, and pair it with the API's canonical result.
/// The proxy's incoming `protocolVersion` is discarded on the way
/// in; the API publishes its own pinned version on the way out.
///
/// Caller (the route layer) stamps the returned `String` onto the
/// outbound HTTP `Mcp-Session-Id` response header so the proxy
/// adopts it.
pub async fn handle_initialize(
    ctx: McpRequestContext,
    _params: InitializeRequestParams,
) -> Result<(InitializeResult, String), McpError> {
    let response = forward(&ctx, server_request::Payload::Initialize).await?;
    match response.payload {
        server_response::Payload::Initialize(r) => {
            let reply = unwrap_rpc(r)?;
            Ok((canonical_initialize_result(), reply.mcp_session_id))
        }
        other => Err(McpError::variant_mismatch("initialize", &other)),
    }
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
    let response = forward(&ctx, server_request::Payload::ToolsList(params)).await?;
    match response.payload {
        server_response::Payload::ToolsList(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("tools_list", &other)),
    }
}

pub async fn handle_tools_call(
    ctx: McpRequestContext,
    params: CallToolRequestParams,
) -> Result<CallToolResult, McpError> {
    let response = forward(&ctx, server_request::Payload::ToolsCall(params)).await?;
    match response.payload {
        server_response::Payload::ToolsCall(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("tools_call", &other)),
    }
}

pub async fn handle_resources_list(
    ctx: McpRequestContext,
    params: ListResourcesRequest,
) -> Result<ListResourcesResult, McpError> {
    let response = forward(&ctx, server_request::Payload::ResourcesList(params)).await?;
    match response.payload {
        server_response::Payload::ResourcesList(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("resources_list", &other)),
    }
}

pub async fn handle_resources_read(
    ctx: McpRequestContext,
    params: ReadResourceRequestParams,
) -> Result<ReadResourceResult, McpError> {
    let response = forward(&ctx, server_request::Payload::ResourcesRead(params)).await?;
    match response.payload {
        server_response::Payload::ResourcesRead(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("resources_read", &other)),
    }
}

// ────────────────────────────────────────────────────────────────
// Session lifecycle (DELETE /objectiveai-mcp)
// ────────────────────────────────────────────────────────────────

pub async fn handle_session_terminate(
    ctx: McpRequestContext,
) -> Result<(), McpError> {
    let response = forward(&ctx, server_request::Payload::SessionTerminate).await?;
    match response.payload {
        server_response::Payload::SessionTerminate => Ok(()),
        other => Err(McpError::variant_mismatch("session_terminate", &other)),
    }
}

// ────────────────────────────────────────────────────────────────
// Internal: build + ship one typed `server_request::Request` over
// the WS, await + return its matching `server_response::Response`.
// Each caller pattern-matches on the response payload to extract
// its method-specific result.
// ────────────────────────────────────────────────────────────────

async fn forward(
    ctx: &McpRequestContext,
    payload: server_request::Payload,
) -> Result<server_response::Response, McpError> {
    let rc = ctx
        .registry
        .get(&ctx.response_id)
        .ok_or_else(|| McpError::no_session(&ctx.response_id))?
        .clone();

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = server_request::Request {
        id: request_id,
        headers: forward_headers(&ctx.headers),
        payload,
    };

    let rx = send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|_| McpError::reverse_channel_closed())?;

    match tokio::time::timeout(FORWARD_TIMEOUT, rx).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(_)) => Err(McpError::reverse_channel_dropped()),
        Err(_) => Err(McpError::reverse_channel_timeout()),
    }
}

/// Project a `JsonRpcResult<R>` from the CLI side into the API's
/// `Result<R, McpError>` shape.
fn unwrap_rpc<R>(
    r: server_response::JsonRpcResult<R>,
) -> Result<R, McpError> {
    match r {
        server_response::JsonRpcResult::Ok { result } => Ok(result),
        server_response::JsonRpcResult::Err {
            code,
            message,
            data,
        } => Err(McpError {
            code,
            message,
            data,
        }),
    }
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
