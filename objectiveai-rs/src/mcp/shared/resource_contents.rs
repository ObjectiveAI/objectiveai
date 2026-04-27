//! Resource content types shared by embedded resources and resource read results.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Base fields shared by all resource contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    /// The URI of this resource.
    pub uri: String,
    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}

/// Text resource contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResourceContents {
    #[serde(flatten)]
    pub base: ResourceContents,
    /// The text of the item.
    pub text: String,
}

/// Binary resource contents (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobResourceContents {
    #[serde(flatten)]
    pub base: ResourceContents,
    /// A base64-encoded string representing the binary data.
    pub blob: String,
}

/// Either text or blob resource contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContentsUnion {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}
