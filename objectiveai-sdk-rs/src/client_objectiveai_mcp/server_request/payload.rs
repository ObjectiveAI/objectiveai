use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every JSON-RPC request the API forwards to the
/// client over the reverse-attach WS. Variant names follow the same
/// snake_case convention `client_request::Payload` uses; the
/// `serde(tag = "type")` discriminator pairs with
/// [`super::super::server_response::Payload`] by name.
///
/// Why these six and only these six: the API never originates any
/// other shape on the wire — see `objectiveai-api/src/objectiveai_mcp/
/// handlers.rs` for the three send sites that build every server
/// request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// POST `initialize`. The proxy's `protocolVersion` doesn't ride
    /// across this hop — the API discards it on the way in and
    /// substitutes its own `canonical_initialize_result` on the way
    /// out. So this variant carries no fields.
    #[schemars(title = "Initialize")]
    Initialize,

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

    /// `DELETE /objectiveai-mcp/{session_id}` — the proxy closing the
    /// session. No body.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate,
}
