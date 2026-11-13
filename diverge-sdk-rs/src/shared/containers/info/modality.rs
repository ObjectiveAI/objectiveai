//! One kind of content, beside text, an agent may take.

use serde::{Deserialize, Serialize};

/// One of the kinds of content a [`Modalities`](super::Modalities)
/// answers for, by name. Snake case on the wire: `"image"`, `"audio"`,
/// `"file"`. Ordered as declared: image, audio, file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// An image.
    Image,
    /// Audio.
    Audio,
    /// A file: bytes that are neither an image nor audio.
    File,
}
