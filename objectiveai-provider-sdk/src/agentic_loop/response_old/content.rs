//! Message content — text, or a sequence of typed parts.

use serde::{Deserialize, Serialize};

/// The content of a message.
///
/// Untagged: plain text serializes as a bare string and multi-part
/// content as an array, so the common case costs nothing on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RichContent {
    /// Plain text.
    Text(String),
    /// Text interleaved with images, audio, video, and files.
    Parts(Vec<RichContentPart>),
}

impl RichContent {
    /// Accumulate a streamed delta into this content.
    ///
    /// Text absorbs text by concatenation. Mixing forms PROMOTES to
    /// parts rather than discarding either side — a turn that starts as
    /// text and later attaches an image is one message, not two.
    pub fn push(&mut self, other: &RichContent) {
        match (&mut *self, other) {
            (RichContent::Text(this), RichContent::Text(that)) => {
                this.push_str(that);
            }
            (RichContent::Text(this), RichContent::Parts(those)) => {
                let mut parts = Vec::with_capacity(1 + those.len());
                parts.push(RichContentPart::Text {
                    text: std::mem::take(this),
                });
                parts.extend(those.iter().cloned());
                *self = RichContent::Parts(parts);
            }
            (RichContent::Parts(these), RichContent::Text(that)) => {
                these.push(RichContentPart::Text { text: that.clone() });
            }
            (RichContent::Parts(these), RichContent::Parts(those)) => {
                these.extend(those.iter().cloned());
            }
        }
    }
}

/// One part of multi-part content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichContentPart {
    /// Text.
    Text { text: String },
    /// An image, by URL or data URL.
    ImageUrl { image_url: ImageUrl },
    /// Audio, inline.
    InputAudio { input_audio: InputAudio },
    /// Video, inline.
    InputVideo { video_url: VideoUrl },
    /// A video, by URL.
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
