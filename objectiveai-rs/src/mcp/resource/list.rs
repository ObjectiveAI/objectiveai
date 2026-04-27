//! Types for resources/list requests and responses.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Parameters for a `resources/list` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    /// An opaque cursor for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// The server's response to a `resources/list` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    /// The list of resources available on the server.
    pub resources: Vec<super::Resource>,
    /// An opaque cursor for fetching the next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
