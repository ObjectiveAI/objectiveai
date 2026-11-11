//! Rich (multimodal) message content.

use rmcp::model;
use serde::Serialize;

/// Rich content for user/assistant messages (supports multimodal input).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
#[serde(untagged)]
pub enum RichContent {
    /// Plain text content.
    Text(String),
    /// Multi-part content (text, images, audio, video, files).
    Parts(Vec<RichContentPart>),
}

/// A part of rich content.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichContentPart {
    /// Text content.
    Text { text: String },
    /// An image URL.
    ImageUrl { image_url: ImageUrl },
    /// Audio input.
    InputAudio { input_audio: InputAudio },
    /// Video input.
    InputVideo { video_url: VideoUrl },
    /// A video URL.
    VideoUrl { video_url: VideoUrl },
    /// A file.
    File { file: File },
}

/// An image URL for multimodal input.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
pub struct ImageUrl {
    /// The URL of the image (can be a data URL or HTTP URL).
    pub url: String,
    /// The detail level for image processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageUrlDetail>,
}

/// Detail level for image processing.
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
pub enum ImageUrlDetail {
    /// Let the model decide the detail level.
    #[serde(rename = "auto")]
    Auto,
    /// Low detail mode (faster, less tokens).
    #[serde(rename = "low")]
    Low,
    /// High detail mode (more accurate, more tokens).
    #[serde(rename = "high")]
    High,
}

/// Audio input for multimodal messages.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
pub struct InputAudio {
    /// Base64-encoded audio data.
    pub data: String,
    /// The audio format (e.g., "wav", "mp3").
    pub format: String,
}

/// A video URL for multimodal input.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
pub struct VideoUrl {
    /// The URL of the video.
    pub url: String,
}

/// A file attachment for multimodal input.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Serialize,
)]
pub struct File {
    /// Base64-encoded file data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The ID of a previously uploaded file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The filename for display purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// A URL to fetch the file from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
}

/// A text block's text, and nothing else: annotations and `_meta`
/// have no OpenRouter home.
impl From<model::TextContent> for RichContentPart {
    fn from(text: model::TextContent) -> Self {
        RichContentPart::Text { text: text.text }
    }
}

/// Compose a base64 data URL from an MCP image's mime and data.
/// `detail` defaults to absent.
impl From<model::ImageContent> for ImageUrl {
    fn from(image: model::ImageContent) -> Self {
        ImageUrl {
            url: format!("data:{};base64,{}", image.mime_type, image.data),
            detail: None,
        }
    }
}

impl From<model::ImageContent> for RichContentPart {
    fn from(image: model::ImageContent) -> Self {
        RichContentPart::ImageUrl {
            image_url: image.into(),
        }
    }
}

/// An MCP audio block's `mime_type`, mapped to the bare format token
/// OpenRouter expects — `"audio/mpeg"` is `"mp3"` — falling back to
/// the mime itself when the mapping does not know it.
impl From<model::AudioContent> for InputAudio {
    fn from(audio: model::AudioContent) -> Self {
        InputAudio {
            format: audio_format(&audio.mime_type),
            data: audio.data,
        }
    }
}

/// The bare format token for an audio mime type.
fn audio_format(mime: &str) -> String {
    mime2ext::mime2ext(mime)
        .map(str::to_string)
        .unwrap_or_else(|| mime.to_string())
}

impl From<model::AudioContent> for RichContentPart {
    fn from(audio: model::AudioContent) -> Self {
        RichContentPart::InputAudio {
            input_audio: audio.into(),
        }
    }
}

/// An embedded resource's contents. Text is text; a blob is
/// dispatched on its mime prefix — an image becomes a data-URL image
/// part, audio becomes an audio part with its bare format token,
/// video becomes a data-URL video part, and anything else is a file,
/// its filename lifted from the URI's trailing path segment.
impl From<model::ResourceContents> for RichContentPart {
    fn from(contents: model::ResourceContents) -> Self {
        match contents {
            model::ResourceContents::TextResourceContents {
                text, ..
            } => RichContentPart::Text { text },
            model::ResourceContents::BlobResourceContents {
                uri,
                mime_type,
                blob,
                ..
            } => {
                let mime = mime_type.as_deref().unwrap_or("");
                if mime.starts_with("image/") {
                    RichContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:{mime};base64,{blob}"),
                            detail: None,
                        },
                    }
                } else if mime.starts_with("audio/") {
                    RichContentPart::InputAudio {
                        input_audio: InputAudio {
                            format: audio_format(mime),
                            data: blob,
                        },
                    }
                } else if mime.starts_with("video/") {
                    RichContentPart::InputVideo {
                        video_url: VideoUrl {
                            url: format!("data:{mime};base64,{blob}"),
                        },
                    }
                } else {
                    let filename = uri
                        .rsplit('/')
                        .next()
                        .filter(|segment| !segment.is_empty())
                        .map(String::from);
                    RichContentPart::File {
                        file: File {
                            file_data: Some(blob),
                            file_id: None,
                            filename,
                            file_url: None,
                        },
                    }
                }
            }
            // Contents a newer MCP defines and this crate does not
            // know: their JSON, which loses nothing and pretends
            // nothing.
            contents => RichContentPart::Text {
                text: serde_json::to_string(&contents).unwrap_or_default(),
            },
        }
    }
}

impl From<model::EmbeddedResource> for RichContentPart {
    fn from(embedded: model::EmbeddedResource) -> Self {
        embedded.resource.into()
    }
}

/// A resource link is a file by URL: OpenRouter's file part carries
/// one natively, so nothing has to be fetched to represent the link.
impl From<model::Resource> for RichContentPart {
    fn from(resource: model::Resource) -> Self {
        RichContentPart::File {
            file: File {
                file_data: None,
                file_id: None,
                filename: Some(resource.name),
                file_url: Some(resource.uri),
            },
        }
    }
}

/// One MCP content block, as the part it is. The wildcard arm covers
/// variants a newer MCP defines and this crate does not know: they
/// pass through as their JSON, which loses nothing and pretends
/// nothing.
impl From<model::ContentBlock> for RichContentPart {
    fn from(block: model::ContentBlock) -> Self {
        match block {
            model::ContentBlock::Text(text) => text.into(),
            model::ContentBlock::Image(image) => image.into(),
            model::ContentBlock::Audio(audio) => audio.into(),
            model::ContentBlock::Resource(embedded) => embedded.into(),
            model::ContentBlock::ResourceLink(resource) => resource.into(),
            block => RichContentPart::Text {
                text: serde_json::to_string(&block).unwrap_or_default(),
            },
        }
    }
}
