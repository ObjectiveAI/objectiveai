use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reply to a [`super::super::client_request::Request`]. Wire shape:
///
/// ```json
/// {"id":"…","type":"ok"}                              // success
/// {"id":"…","type":"error","code":404,"message":…}    // failure
/// ```
///
/// The internally-tagged enum keeps the wire flat — `id`, `type`,
/// and any extra fields sit side-by-side on the JSON object — while
/// keeping the schema clean (no `anyOf`-with-sibling-properties
/// structural quirk that flattening an inner Result enum would
/// otherwise produce).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_response.Response")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// Empty success — the request was accepted.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.Ok")]
    Ok {
        /// Matches the `id` of the
        /// [`super::super::client_request::Request`] this response is for.
        id: String,
    },
    /// The request failed.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.Error")]
    Error {
        /// Matches the `id` of the
        /// [`super::super::client_request::Request`] this response is for.
        id: String,
        code: u16,
        message: serde_json::Value,
    },

    /// Reply to [`super::super::client_request::Payload::ListTools`] —
    /// the proxy's normal aggregated `tools/list` result (or an error),
    /// the same shape its HTTP endpoint returns.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.ListTools")]
    ListTools {
        id: String,
        #[serde(flatten)]
        result: super::super::server_response::JsonRpcResult<crate::mcp::tool::ListToolsResult>,
    },

    /// Reply to [`super::super::client_request::Payload::CallTool`].
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.CallTool")]
    CallTool {
        id: String,
        #[serde(flatten)]
        result: super::super::server_response::JsonRpcResult<crate::mcp::tool::CallToolResult>,
    },

    /// Reply to [`super::super::client_request::Payload::ListResources`].
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.ListResources")]
    ListResources {
        id: String,
        #[serde(flatten)]
        result: super::super::server_response::JsonRpcResult<crate::mcp::resource::ListResourcesResult>,
    },

    /// Reply to [`super::super::client_request::Payload::ReadResource`].
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.ReadResource")]
    ReadResource {
        id: String,
        #[serde(flatten)]
        result: super::super::server_response::JsonRpcResult<crate::mcp::resource::ReadResourceResult>,
    },

    /// Reply to [`super::super::client_request::Payload::ListServers`] —
    /// the proxy's connected upstream MCP servers and their metadata.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.ListServers")]
    ListServers {
        id: String,
        #[serde(flatten)]
        result: super::super::server_response::JsonRpcResult<crate::mcp::server::ListServersResult>,
    },
}

impl Response {
    /// The correlation id present on every variant.
    pub fn id(&self) -> &str {
        match self {
            Response::Ok { id } => id,
            Response::Error { id, .. } => id,
            Response::ListTools { id, .. } => id,
            Response::CallTool { id, .. } => id,
            Response::ListResources { id, .. } => id,
            Response::ReadResource { id, .. } => id,
            Response::ListServers { id, .. } => id,
        }
    }
}
