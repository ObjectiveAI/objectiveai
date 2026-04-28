//! Types for tools/list requests and responses.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for a `tools/list` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ListToolsRequest")]
pub struct ListToolsRequest {
    /// An opaque cursor for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cursor: Option<String>,
}

/// The server's response to a `tools/list` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ListToolsResult")]
pub struct ListToolsResult {
    /// The list of tools available on the server.
    pub tools: Vec<super::Tool>,
    /// An opaque cursor for fetching the next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
