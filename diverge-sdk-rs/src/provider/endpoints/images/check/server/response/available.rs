//! The affirmative answer.

use serde::{Deserialize, Serialize};

/// The provider can supply the image.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Available {
    /// The discriminator.
    pub r#type: AvailableType,
}

/// [`Available`]'s discriminator.
///
/// One variant, and the reason [`Response`](super::Response) can be
/// untagged: [`Unavailable`](super::Unavailable) cannot produce this
/// value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AvailableType {
    #[serde(rename = "available")]
    #[default]
    Available,
}
