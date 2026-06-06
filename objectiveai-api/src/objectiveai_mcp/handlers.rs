//! Typed delegate functions, one per MCP route. Every forwarding
//! delegate wraps the same primitive: build a typed
//! `server_request::Payload`, ship it to the CLI via
//! `send_server_request`, await the matching `server_response`,
//! pattern-match the expected `server_response::Payload` variant
//! (`JsonRpcResult::Ok` → typed result, `Err` → propagate as
//! [`McpError`]).
//!
//! The route layer parses `mcp_kind` off the URL path and threads it
//! into every delegate so the CLI can dispatch to the right per-MCP
//! handler. Plugin args (URL query string on the inbound POST) ride
//! into `handle_initialize` and only there — non-`initialize`
//! requests reuse the cached upstream connection on the CLI side.

use super::send::send_server_request;
use crate::objectiveai_mcp::context::McpRequestContext;
use axum::http::HeaderMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use objectiveai_sdk::client_objectiveai_mcp::server_request::InitializeRequest;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use objectiveai_sdk::mcp::initialize_result::InitializeResult;
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams,
    ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};
use std::time::Duration;

/// How long to wait for a `server_response` over the WS before failing
/// the request as a gateway timeout. Mirrors the SDK conduit endpoint's
/// `REVERSE_CHANNEL_TIMEOUT`.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(30);

/// Common error shape every delegate returns. The route layer renders
/// this into either a JSON-RPC error envelope (under `POST /…`) or an
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
        P::SessionTerminate(_) => "session_terminate",
    }
}

// ────────────────────────────────────────────────────────────────
// JSON-RPC method delegates (POST on either per-MCP route)
// ────────────────────────────────────────────────────────────────

/// `initialize` — forward to the CLI with the path-extracted
/// [`McpKind`] and URL-query-string-parsed plugin args; return the
/// upstream's verbatim `InitializeResult` plus its native
/// `Mcp-Session-Id`. The CLI is a pure medium — it doesn't
/// synthesize capabilities, doesn't pin a protocol version, doesn't
/// name itself. Whatever the upstream MCP server (the local
/// `objectiveai-mcp` HTTP server or the plugin's MCP subprocess)
/// reported, the proxy sees.
///
/// Caller (the route layer) stamps the returned `String` onto the
/// outbound HTTP `Mcp-Session-Id` response header so the proxy
/// adopts it as the session id for this particular per-MCP upstream.
pub async fn handle_initialize(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    args: IndexMap<String, Option<String>>,
) -> Result<(InitializeResult, String), McpError> {
    let response = forward(
        &ctx,
        mcp_kind,
        server_request::Payload::Initialize(InitializeRequest { args }),
    )
    .await?;
    match response.payload {
        server_response::Payload::Initialize(r) => {
            let reply = unwrap_rpc(r)?;
            Ok((reply.result, reply.mcp_session_id))
        }
        other => Err(McpError::variant_mismatch("initialize", &other)),
    }
}

pub async fn handle_ping(_ctx: McpRequestContext) -> Result<(), McpError> {
    // Local. The route layer 404'd already if the response_id was
    // bogus; we just confirm liveness. `ping` is not per-MCP — it
    // answers on either route prefix without forwarding.
    Ok(())
}

pub async fn handle_tools_list(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: ListToolsRequest,
) -> Result<ListToolsResult, McpError> {
    let response = forward(&ctx, mcp_kind, server_request::Payload::ToolsList(params)).await?;
    match response.payload {
        server_response::Payload::ToolsList(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("tools_list", &other)),
    }
}

pub async fn handle_tools_call(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: CallToolRequestParams,
) -> Result<CallToolResult, McpError> {
    let response = forward(&ctx, mcp_kind, server_request::Payload::ToolsCall(params)).await?;
    match response.payload {
        server_response::Payload::ToolsCall(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("tools_call", &other)),
    }
}

pub async fn handle_resources_list(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: ListResourcesRequest,
) -> Result<ListResourcesResult, McpError> {
    let response = forward(
        &ctx,
        mcp_kind,
        server_request::Payload::ResourcesList(params),
    )
    .await?;
    match response.payload {
        server_response::Payload::ResourcesList(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("resources_list", &other)),
    }
}

pub async fn handle_resources_read(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
    params: ReadResourceRequestParams,
) -> Result<ReadResourceResult, McpError> {
    let response = forward(
        &ctx,
        mcp_kind,
        server_request::Payload::ResourcesRead(params),
    )
    .await?;
    match response.payload {
        server_response::Payload::ResourcesRead(r) => unwrap_rpc(r),
        other => Err(McpError::variant_mismatch("resources_read", &other)),
    }
}

// ────────────────────────────────────────────────────────────────
// Session lifecycle (DELETE on either per-MCP route)
// ────────────────────────────────────────────────────────────────

pub async fn handle_session_terminate(
    ctx: McpRequestContext,
    mcp_kind: McpKind,
) -> Result<(), McpError> {
    let response = forward(&ctx, mcp_kind, server_request::Payload::SessionTerminate).await?;
    match response.payload {
        server_response::Payload::SessionTerminate(r) => unwrap_rpc(r),
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
    mcp_kind: McpKind,
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
        mcp_kind,
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

/// Copy inbound headers for forwarding, dropping hop-by-hop ones
/// only. **Every `X-OBJECTIVEAI-*` header passes through unchanged**
/// — they're load-bearing downstream of routing:
///
/// - The six transient headers (`AGENT-INSTANCE-HIERARCHY`,
///   `AGENT-ID`, `AGENT-FULL-ID`, `AGENT-REMOTE`, `RESPONSE-ID`,
///   `RESPONSE-IDS`) feed the CLI conduit's `require_transient`
///   check, populate `ctx.config.{agent_*,response_*}`, and project
///   onto every subprocess env via `apply_config_env`.
/// - `RESPONSE-ID` doubles as the api's own routing key here, but
///   stripping it before forwarding would break the CLI's
///   `require_transient` (and every consumer of
///   `ctx.config.response_id` downstream).
/// - `ARGUMENTS` carries the per-plugin JSON-serialized argument
///   map declared on `client_objectiveai_mcp.plugins[].mcp_servers[].arguments`;
///   the cli's `dial_plugin_upstream` reads them via the typed
///   `Initialize { args }` payload on the `initialize` POST and
///   via the raw header on every other request that touches the
///   plugin subprocess env (`OAI_*_ARG_*`).
/// - `AUTHORIZATION`, `SIGNATURE`, `MCP-CONFIG`, `TOOLS-ALLOWED`
///   are likewise downstream-bound — there is no api-level
///   consumer for any of them and the cli or plugin needs them
///   verbatim.
///
/// `Mcp-Session-Id` also passes through — that's the standard MCP
/// transport identifier minted by the upstream server, threaded
/// end-to-end so the cli's conduit can key its per-upstream
/// `connections` DashMap against the same id the proxy sees.
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
            );
            if drop {
                return None;
            }
            Some((name.to_string(), v.to_str().ok()?.to_string()))
        })
        .collect()
}
