use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of request shapes the client-app layer can push down
/// the reverse-attach channel.
///
/// Despite the module name, payloads flow in BOTH directions:
/// - **API → client**: `AgentCompletionNotify` (the API pushes a
///   user message into a running agent completion).
/// - **client → API**: `McpListChanged` (the CLI's upstream
///   `mcp::Connection` fired
///   `notifications/{tools,resources}/list_changed` and the API
///   re-emits it as an SSE event on the matching per-MCP GET
///   stream — `/objectiveai` or `/{owner}/{name}/{ver}/{mcp}`,
///   routed by `X-OBJECTIVEAI-RESPONSE-ID`).
///
/// The wire envelope's `id` field always belongs to whichever side
/// originated the request; the receiver's `client_response::Response`
/// echoes the same `id`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    #[schemars(title = "AgentCompletionNotify")]
    AgentCompletionNotify(
        crate::agent::completions::request::AgentCompletionNotifyParams,
    ),
    /// The CLI's upstream `mcp::Connection` for `mcp_kind` fired
    /// `notifications/<kind>/list_changed`. The API dispatches this
    /// onto its per-`(response_id, McpKind)` broadcast so the
    /// matching MCP GET-SSE listener sees a standard MCP
    /// notification frame.
    #[schemars(title = "McpListChanged")]
    McpListChanged(McpListChanged),
}

/// Payload for [`Payload::McpListChanged`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.McpListChanged")]
pub struct McpListChanged {
    /// Which CLI-hosted MCP server fired the list-changed
    /// notification. The API uses this to look up the right
    /// per-MCP SSE broadcast and republish a standard MCP
    /// notification frame to that upstream's proxy subscriber.
    pub mcp_kind: super::super::McpKind,
    /// Which catalog changed.
    pub kind: McpListChangedKind,
}

/// Distinguishes `tools/list_changed` from `resources/list_changed`.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash,
)]
#[schemars(rename = "client_objectiveai_mcp.client_request.McpListChangedKind")]
#[serde(rename_all = "snake_case")]
pub enum McpListChangedKind {
    Tools,
    Resources,
}

impl McpListChangedKind {
    /// JSON-RPC method name MCP uses on the wire for this notification
    /// kind. Used by the API's GET-SSE handler when it emits the
    /// translated frame to subscribers.
    pub fn method(&self) -> &'static str {
        match self {
            McpListChangedKind::Tools => "notifications/tools/list_changed",
            McpListChangedKind::Resources => {
                "notifications/resources/list_changed"
            }
        }
    }
}
