//! Types for resources/list requests and responses.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for a `resources/list` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.resource.ListResourcesRequest")]
pub struct ListResourcesRequest {
    /// An opaque cursor for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cursor: Option<String>,
}

/// The server's response to a `resources/list` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.resource.ListResourcesResult")]
pub struct ListResourcesResult {
    /// The list of resources available on the server.
    pub resources: Vec<super::Resource>,
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
