use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every JSON-RPC request the API forwards to the
/// client over the reverse-attach WS. Variant names follow the same
/// snake_case convention `client_request::Payload` uses; the
/// `serde(tag = "type")` discriminator pairs with
/// [`super::super::server_response::Payload`] by name.
///
/// Which CLI-hosted MCP server the request targets rides on the
/// enclosing [`super::Request`] envelope's `mcp_kind` field, not
/// inside the variant — every variant maps 1:1 to a JSON-RPC method
/// regardless of which MCP is on the receiving end.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// POST `initialize`. The proxy's `protocolVersion` doesn't ride
    /// across this hop — the API discards it on the way in and
    /// substitutes its own `canonical_initialize_result` on the way
    /// out. The variant carries the plugin arguments the CLI needs at
    /// dial time (parsed by the API off the URL query string).
    #[schemars(title = "Initialize")]
    Initialize(InitializeRequest),

    /// POST `tools/list`.
    #[schemars(title = "ToolsList")]
    ToolsList(crate::mcp::tool::ListToolsRequest),

    /// POST `tools/call`.
    #[schemars(title = "ToolsCall")]
    ToolsCall(crate::mcp::tool::CallToolRequestParams),

    /// POST `resources/list`.
    #[schemars(title = "ResourcesList")]
    ResourcesList(crate::mcp::resource::ListResourcesRequest),

    /// POST `resources/read`.
    #[schemars(title = "ResourcesRead")]
    ResourcesRead(crate::mcp::resource::ReadResourceRequestParams),

    /// `DELETE` on the routed MCP URL — the proxy closing the
    /// session. No body.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate,
}

/// Parameters for [`Payload::Initialize`].
///
/// Carries plugin arguments lifted off the inbound URL's query
/// string (`?key=value&flag` → `{"key": Some("value"), "flag": None}`).
/// Empty for [`super::super::McpKind::ObjectiveAi`] (the primary
/// upstream takes no args).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.InitializeRequest")]
pub struct InitializeRequest {
    /// Plugin arguments the CLI passes through to
    /// `<plugin> mcp <mcp_name> begin --<key> [value]`. `None` value
    /// means presence-only flag (`--key`); `Some(v)` means `--key v`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub args: IndexMap<String, Option<String>>,
}
