//! Resource content types shared by embedded resources and resource read results.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Base fields shared by all resource contents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.shared.ResourceContents")]
pub struct ResourceContents {
    /// The URI of this resource.
    pub uri: String,
    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}

/// Text resource contents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.shared.TextResourceContents")]
pub struct TextResourceContents {
    #[serde(flatten)]
    pub base: ResourceContents,
    /// The text of the item.
    pub text: String,
}

/// Binary resource contents (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.shared.BlobResourceContents")]
pub struct BlobResourceContents {
    #[serde(flatten)]
    pub base: ResourceContents,
    /// A base64-encoded string representing the binary data.
    pub blob: String,
}

/// Either text or blob resource contents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.shared.ResourceContentsUnion")]
pub enum ResourceContentsUnion {
    #[schemars(title = "Text")]
    Text(TextResourceContents),
    #[schemars(title = "Blob")]
    Blob(BlobResourceContents),
}
