//! Image types for completion responses.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response;
use serde::Deserialize;

/// An image in a agent completion response.
///
/// Used when models generate images as part of their response.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Image {
    /// Image provided as a URL.
    ImageUrl {
        /// The image URL details.
        image_url: ImageUrl,
    },
}

/// URL reference to an image.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ImageUrl {
    /// URL where the image can be accessed.
    pub url: String,
}

impl Image {
    /// Append this image as an image chunk — when it can be one.
    ///
    /// The chunk carries base64 data and a mime type, so only a
    /// `data:<mime>;base64,<payload>` URL converts; any other URL is
    /// dropped, because a conversion does no I/O and has nothing else
    /// to put in the chunk. OpenRouter returns data URLs in practice.
    pub fn into_chunks(self, chunks: &mut Vec<response::AgenticLoopChunk>) {
        let Image::ImageUrl { image_url } = self;
        let Some(rest) = image_url.url.strip_prefix("data:") else {
            return;
        };
        let Some((mime_type, data)) = rest.split_once(";base64,") else {
            return;
        };
        chunks.push(response::AgenticLoopChunk::AssistantImageContent(
            response::AssistantImageContentChunk {
                r#type: Default::default(),
                parent_tool_call_id: None,
                inner: rmcp::model::ImageContent::new(data, mime_type),
            },
        ));
    }
}
