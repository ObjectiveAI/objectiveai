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

/// Returned from [`TryFrom<&ImageUrl>`] for `ImageContent` when the
/// source URL is not a `data:<mime>;base64,<payload>` URL — i.e. an
/// http(s) or unknown-scheme URL that can't be inlined as base64.
#[derive(Debug, thiserror::Error)]
#[error("image url is not a base64 data URL: {url}")]
pub struct ImageUrlNotDataUrl {
    pub url: String,
}

impl TryFrom<crate::agent::completions::message::ImageUrl> for ImageContent {
    type Error = ImageUrlNotDataUrl;

    fn try_from(
        image_url: crate::agent::completions::message::ImageUrl,
    ) -> Result<Self, Self::Error> {
        let parsed = image_url
            .url
            .strip_prefix("data:")
            .and_then(|rest| rest.split_once(";base64,"))
            .map(|(mime, data)| (mime.to_string(), data.to_string()));
        match parsed {
            Some((mime_type, data)) => Ok(Self {
                data,
                mime_type,
                annotations: None,
                _meta: None,
            }),
            None => Err(ImageUrlNotDataUrl { url: image_url.url }),
        }
    }
}
