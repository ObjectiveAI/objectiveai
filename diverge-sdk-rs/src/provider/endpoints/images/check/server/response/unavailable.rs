//! The negative answer.

use serde::{Deserialize, Serialize};

/// The provider cannot supply the image.
///
/// Deliberately says nothing about why. "I do not have it" and "I have
/// it but will not serve it to you" are the same answer from where the
/// caller stands, and distinguishing them would tell an unauthorized
/// caller that a private image exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Unavailable {
    /// The discriminator.
    pub r#type: UnavailableType,
}

/// [`Unavailable`]'s discriminator.
///
/// One variant, and the reason [`Response`](super::Response) can be
/// untagged: [`Available`](super::Available) cannot produce this
/// value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum UnavailableType {
    #[serde(rename = "unavailable")]
    #[default]
    Unavailable,
}
