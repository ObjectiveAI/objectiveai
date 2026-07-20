use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of request shapes the client-app layer can push down
/// the reverse-attach channel.
///
/// Currently a single client → API variant: `McpListChanged`. The CLI's
/// upstream `mcp::Connection` fired
/// `notifications/{tools,resources}/list_changed` and the API
/// re-emits it as an SSE event on the matching per-MCP GET stream —
/// `/objectiveai` or `/{owner}/{name}/{ver}/{mcp}`, routed by
/// `X-OBJECTIVEAI-RESPONSE-ID`.
///
/// The wire envelope's `id` field always belongs to whichever side
/// originated the request; the receiver's `client_response::Response`
/// echoes the same `id`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// The CLI's upstream `mcp::Connection` for `mcp_kind` fired
    /// `notifications/<kind>/list_changed`. The API dispatches this
    /// onto its per-`(response_id, McpKind)` broadcast so the
    /// matching MCP GET-SSE listener sees a standard MCP
    /// notification frame.
    #[schemars(title = "McpListChanged")]
    McpListChanged(McpListChanged),

    /// Run the proxy's aggregated `tools/list` for `response_id` — the
    /// same operation the proxy's HTTP `tools/list` endpoint performs
    /// (fan out to every upstream in that session). Session-scoped, so
    /// no `mcp_kind`; carries the normal MCP `tools/list` params.
    #[schemars(title = "ListTools")]
    ListTools {
        response_id: String,
        /// Restrict the listing to the single server with this name (the
        /// proxy's routing prefix). `None` fans out to every upstream.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
        #[serde(flatten)]
        params: crate::mcp::tool::ListToolsRequest,
    },

    /// Run the proxy's aggregated `tools/call` for `response_id` (routes
    /// by tool-name prefix to the owning upstream). Unlike the HTTP
    /// `tools/call`, this path does NOT consult the queue delegate.
    #[schemars(title = "CallTool")]
    CallTool {
        response_id: String,
        #[serde(flatten)]
        params: crate::mcp::tool::CallToolRequestParams,
    },

    /// Run the proxy's aggregated `resources/list` for `response_id`.
    #[schemars(title = "ListResources")]
    ListResources {
        response_id: String,
        /// Restrict the listing to the single server with this name (the
        /// proxy's routing prefix). `None` fans out to every upstream.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        name: Option<String>,
        #[serde(flatten)]
        params: crate::mcp::resource::ListResourcesRequest,
    },

    /// List the proxy's connected upstream MCP servers for `response_id`,
    /// with each server's initialize metadata. A proxy-local aggregate
    /// answered from its in-memory connection set — no MCP params, no
    /// upstream round-trip.
    #[schemars(title = "ListServers")]
    ListServers { response_id: String },

    /// Run the proxy's `resources/read` for `response_id` (routes by URI
    /// prefix to the owning upstream).
    #[schemars(title = "ReadResource")]
    ReadResource {
        response_id: String,
        #[serde(flatten)]
        params: crate::mcp::resource::ReadResourceRequestParams,
    },
}

/// Payload for [`Payload::McpListChanged`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.McpListChanged")]
pub struct McpListChanged {
    /// Which client-side MCP server fired the list-changed
    /// notification. The API uses this to look up the right
    /// per-MCP SSE broadcast and republish a standard MCP
    /// notification frame to that upstream's proxy subscriber.
    pub mcp_kind: super::super::McpKind,
    /// Which catalog changed.
    pub kind: McpListChangedKind,
    /// The response id of the session whose upstream fired — lets the
    /// proxy disambiguate identical kinds across swarm slots sharing
    /// one reverse channel. Optional + defaulted for old senders; the
    /// daemon's relay (the system's sole emitter) always populates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_id: Option<String>,
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
