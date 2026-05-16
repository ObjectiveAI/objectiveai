//! Image content block.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Image content (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ImageContent")]
pub struct ImageContent {
    /// The base64-encoded image data.
    pub data: String,
    /// The MIME type of the image.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub annotations: Option<super::super::shared::Annotations>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
