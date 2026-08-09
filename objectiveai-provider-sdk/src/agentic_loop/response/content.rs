//! Message content — text, or a sequence of typed parts.

use serde::{Deserialize, Serialize};

/// The content of a message.
///
/// Untagged, so plain text serializes as a bare string and multi-part
/// content as an array. The overwhelmingly common case — text — costs
/// nothing on the wire, and no consumer has to unwrap a single-element
/// array to read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// Plain text.
    Text(String),
    /// Text interleaved with images, audio, video, and files.
    Parts(Vec<ContentPart>),
}

/// One part of multi-part content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text.
    Text { text: String },
    /// An image, by URL or `data:` URL.
    ImageUrl { image_url: ImageUrl },
    /// Audio, inline.
    InputAudio { input_audio: InputAudio },
    /// A video, by URL or `data:` URL.
    VideoUrl { video_url: VideoUrl },
    /// A file, inline or by id.
    File { file: File },
}

/// An image reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImageUrl {
    /// An HTTP URL or a `data:` URL.
    pub url: String,
    /// How much resolution the model should spend on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageUrlDetail>,
}

/// The resolution an image should be processed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageUrlDetail {
    /// Let the provider decide.
    #[default]
    Auto,
    /// Cheaper, coarser.
    Low,
    /// More expensive, finer.
    High,
}

/// Inline audio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InputAudio {
    /// Base64-encoded audio.
    pub data: String,
    /// The container format, e.g. `wav` or `mp3`.
    pub format: String,
}

/// A video reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VideoUrl {
    /// An HTTP URL or a `data:` URL.
    pub url: String,
}

/// A file, supplied inline or referenced by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct File {
    /// Base64-encoded contents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The id of a previously uploaded file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// A display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}
