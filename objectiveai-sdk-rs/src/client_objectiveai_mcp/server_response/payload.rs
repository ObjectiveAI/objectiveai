use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every reply the CLI can send back to the API in
/// answer to a [`super::super::server_request::Payload`]. Variant
/// names pair 1:1 with the request side; the typed `result` /
/// `error` shape per JSON-RPC method is captured in
/// [`JsonRpcResult`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// Reply to
    /// [`super::super::server_request::Payload::Initialize`]. On
    /// success carries the upstream MCP session id the API stamps
    /// onto its outbound `Mcp-Session-Id` response header so the
    /// proxy adopts it. On failure (dial or aggregate-build error)
    /// carries a JSON-RPC error envelope the API translates into its
    /// own outbound error.
    #[schemars(title = "Initialize")]
    Initialize(JsonRpcResult<InitializeReply>),

    /// Reply to [`super::super::server_request::Payload::ToolsList`].
    #[schemars(title = "ToolsList")]
    ToolsList(JsonRpcResult<crate::mcp::tool::ListToolsResult>),

    /// Reply to [`super::super::server_request::Payload::ToolsCall`].
    #[schemars(title = "ToolsCall")]
    ToolsCall(JsonRpcResult<crate::mcp::tool::CallToolResult>),

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesList`].
    #[schemars(title = "ResourcesList")]
    ResourcesList(JsonRpcResult<crate::mcp::resource::ListResourcesResult>),

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesRead`].
    #[schemars(title = "ResourcesRead")]
    ResourcesRead(JsonRpcResult<crate::mcp::resource::ReadResourceResult>),

    /// Acknowledges
    /// [`super::super::server_request::Payload::SessionTerminate`].
    /// On success carries the unit value (no body); on failure
    /// carries the upstream-delete error so the proxy sees a
    /// non-2xx and can retry.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate(JsonRpcResult<()>),
}

/// The successful `Initialize` payload — the upstream's verbatim
/// `InitializeResult` plus the native `Mcp-Session-Id` the CLI got
/// back from dialing the actual MCP server. The API forwards both
/// to the proxy: the result as the JSON-RPC body, the session id as
/// the `Mcp-Session-Id` response header. The CLI is a pure medium —
/// it doesn't synthesize capabilities, doesn't name itself, doesn't
/// pin a protocol version. Whatever the upstream MCP advertised is
/// what the proxy sees.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.InitializeReply")]
pub struct InitializeReply {
    /// Upstream's native `Mcp-Session-Id`. One per CLI-hosted MCP
    /// server — no aggregation, no encoding.
    pub mcp_session_id: String,
    /// The upstream's verbatim `InitializeResult` (capabilities,
    /// server info, protocol version). Returned as-is to the proxy.
    pub result: crate::mcp::initialize_result::InitializeResult,
}

/// JSON-RPC result/error shape for every typed method. Mirrors
/// the wire shape upstream MCP servers return (`{result: …}` on
/// success, `{error: {code, message, data?}}` on failure) but typed
/// at the SDK level instead of buried inside a `serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.JsonRpcResult.{R}", bound = "R: JsonSchema")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JsonRpcResult<R> {
    /// Method returned a typed result.
    #[schemars(title = "Ok")]
    Ok { result: R },
    /// Method returned a JSON-RPC error envelope.
    #[schemars(title = "Err")]
    Err {
        code: i64,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        data: Option<serde_json::Value>,
    },
}
