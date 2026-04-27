//! Audio content block.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Audio content (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContent {
    /// The base64-encoded audio data.
    pub data: String,
    /// The MIME type of the audio.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<super::super::shared::Annotations>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
