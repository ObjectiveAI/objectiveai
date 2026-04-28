//! Types for resources/read requests and responses.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for a `resources/read` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.resource.ReadResourceRequestParams")]
pub struct ReadResourceRequestParams {
    /// The URI of the resource to read.
    pub uri: String,
}

/// The server's response to a `resources/read` request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.resource.ReadResourceResult")]
pub struct ReadResourceResult {
    /// The contents of the resource.
    pub contents: Vec<super::super::shared::ResourceContentsUnion>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
