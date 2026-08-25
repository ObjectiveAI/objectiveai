//! Rich (multimodal) message content.

use serde::{Deserialize, Serialize};

/// Rich content for user/assistant messages (supports multimodal input).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
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
    Deserialize,
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
    Deserialize,
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
    Deserialize,
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
    Deserialize,
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
    Deserialize,
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
    Deserialize,
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
