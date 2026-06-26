use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One reverse-attach request the API has shipped to the calling
/// client. The proxy's HTTP method (`POST` for the five JSON-RPC
/// methods, `DELETE` for session terminate) is implicit in the
/// [`super::Payload`] variant; the JSON-RPC `{jsonrpc, id, method,
/// params}` envelope is unwrapped into the typed variant payload.
///
/// Which CLI-hosted MCP server the request targets rides as
/// `mcp_kind` on the envelope. The API parses this off the inbound
/// URL path (`/objectiveai` → [`super::super::McpKind::ObjectiveAi`];
/// `/{owner}/{name}/{version}/{mcp}` → [`super::super::McpKind::Plugin`])
/// before forwarding.
///
/// Wire shape (envelope is `{id, mcp_kind, headers?, type, …variant
/// fields…}` after the `#[serde(flatten)]` on `payload`):
///
/// ```json
/// {
///   "id":"…",
///   "mcp_kind":{"type":"objective_ai"},
///   "headers":{"Mcp-Session-Id":"…"},
///   "type":"tools_list",
///   "cursor":"…"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Request")]
pub struct Request {
    /// Server-minted correlation id. Echoed by the matching
    /// [`super::super::server_response::Response`].
    pub id: String,
    /// Verbatim copy of the headers the proxy sent on its HTTP
    /// request to the API. The CLI conduit reads several custom
    /// `X-OBJECTIVEAI-*` routing headers + `Mcp-Session-Id` off this
    /// map; protocol-level headers (Host, Content-Length, …) the API
    /// already stripped on its way in.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub headers: IndexMap<String, String>,
    /// The typed request variant. The MCP-routed variants carry
    /// `mcp_kind` inside the variant itself (see [`super::Payload`]);
    /// non-MCP variants don't.
    #[serde(flatten)]
    pub payload: super::Payload,
}

impl Request {
    /// Which CLI-hosted MCP server this request targets. `Some` for
    /// the MCP-routed variants (`Initialize` / `ToolsList` /
    /// `ToolsCall` / `ResourcesList` / `ResourcesRead` /
    /// `SessionTerminate`); `None` for non-MCP variants
    /// (`ReadMessageQueue` / `ClearMessageQueue`) which hit the CLI's
    /// own local state. Delegates to [`super::Payload::mcp_kind`].
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        self.payload.mcp_kind()
    }
}
