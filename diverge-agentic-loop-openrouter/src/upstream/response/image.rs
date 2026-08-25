//! Image types for completion responses.

use serde::{Deserialize, Serialize};

/// An image in a agent completion response.
///
/// Used when models generate images as part of their response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Image {
    /// Image provided as a URL.
    ImageUrl {
        /// The image URL details.
        image_url: ImageUrl,
    },
}

/// URL reference to an image.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageUrl {
    /// URL where the image can be accessed.
    pub url: String,
}
