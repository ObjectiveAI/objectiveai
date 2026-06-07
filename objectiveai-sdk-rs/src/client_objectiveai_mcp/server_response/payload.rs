use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every reply the CLI can send back to the API in
/// answer to a [`super::super::server_request::Payload`]. Variant
/// names pair 1:1 with the request side; the typed `result` /
/// `error` shape per JSON-RPC method is captured in
/// [`JsonRpcResult`].
///
/// MCP-routed variants echo `mcp_kind` on the variant itself
/// (lets the API sanity-check routing). Non-MCP variants
/// (`ReadMessageQueue` / `ClearMessageQueue`) don't carry
/// `mcp_kind` — they never had one to echo. Use
/// [`Payload::mcp_kind`] to retrieve it generically.
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
    Initialize {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<InitializeReply>,
    },

    /// Reply to [`super::super::server_request::Payload::ToolsList`].
    #[schemars(title = "ToolsList")]
    ToolsList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::tool::ListToolsResult>,
    },

    /// Reply to [`super::super::server_request::Payload::ToolsCall`].
    #[schemars(title = "ToolsCall")]
    ToolsCall {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::tool::CallToolResult>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesList`].
    #[schemars(title = "ResourcesList")]
    ResourcesList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::resource::ListResourcesResult>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesRead`].
    #[schemars(title = "ResourcesRead")]
    ResourcesRead {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::resource::ReadResourceResult>,
    },

    /// Acknowledges
    /// [`super::super::server_request::Payload::SessionTerminate`].
    /// On success carries the unit value (no body); on failure
    /// carries the upstream-delete error so the proxy sees a
    /// non-2xx and can retry.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<()>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ReadMessageQueue`].
    /// On success carries every matching queue row's id + body in
    /// oldest-first order; on failure surfaces the local storage
    /// error so the API can decide whether to retry. Non-MCP — no
    /// `mcp_kind` to echo.
    #[schemars(title = "ReadMessageQueue")]
    ReadMessageQueue(JsonRpcResult<ReadMessageQueueResult>),

    /// Acknowledges
    /// [`super::super::server_request::Payload::ClearMessageQueue`].
    /// Unit on success; storage error on failure. No row-count is
    /// returned — unknown ids are silently absorbed. Non-MCP — no
    /// `mcp_kind` to echo.
    #[schemars(title = "ClearMessageQueue")]
    ClearMessageQueue(JsonRpcResult<()>),
}

impl Payload {
    /// Which CLI-hosted MCP server produced this reply. `Some` for
    /// MCP-routed variants (echoes the request's `mcp_kind`); `None`
    /// for `ReadMessageQueue` and `ClearMessageQueue`.
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        match self {
            Payload::Initialize { mcp_kind, .. }
            | Payload::ToolsList { mcp_kind, .. }
            | Payload::ToolsCall { mcp_kind, .. }
            | Payload::ResourcesList { mcp_kind, .. }
            | Payload::ResourcesRead { mcp_kind, .. }
            | Payload::SessionTerminate { mcp_kind, .. } => Some(mcp_kind.clone()),
            Payload::ReadMessageQueue(_) | Payload::ClearMessageQueue(_) => None,
        }
    }
}

/// Successful payload for
/// [`Payload::ReadMessageQueue`].
///
/// Entries are oldest-first (`prompts.id ASC`) — same ordering the
/// in-process drain emits. Feed each `id` back into
/// [`super::super::server_request::ClearMessageQueueRequest::ids`]
/// once the content has been consumed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.ReadMessageQueueResult")]
pub struct ReadMessageQueueResult {
    pub entries: Vec<ReadMessageQueueEntry>,
}

/// One row in a [`ReadMessageQueueResult`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.ReadMessageQueueEntry")]
pub struct ReadMessageQueueEntry {
    /// `prompts.id` — opaque integer the caller feeds back into
    /// [`super::super::server_request::ClearMessageQueueRequest`] to
    /// drop the row.
    pub id: i64,
    /// Reconstructed body. Single-text-part rows collapse to
    /// [`crate::agent::completions::message::RichContent::Text`];
    /// multi-part rows stay as
    /// [`crate::agent::completions::message::RichContent::Parts`].
    pub content: crate::agent::completions::message::RichContent,
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
