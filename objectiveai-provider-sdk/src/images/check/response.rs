//! The image check response.

use serde::{Deserialize, Serialize};

/// Whether a provider can supply the image that was asked about.
///
/// Untagged, with each variant's payload carrying its own `type`
/// constant — the same discipline the agentic loop chunks use. serde
/// has no tag of its own to read, so the answer goes on the wire as
/// itself rather than as a wrapper around itself.
///
/// Two variants rather than a `bool`, because only one of the two
/// answers has anything more to say. An available image has terms
/// attached to it; an unavailable one is just absent. A boolean would
/// force everything that qualifies availability to sit beside it as an
/// optional field that is meaningless when the answer is no.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageCheckResponse {
    /// The provider can supply it.
    Available(Available),
    /// It cannot.
    Unavailable(Unavailable),
}

/// The provider can supply the image.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Available {
    /// The discriminator.
    pub r#type: AvailableType,
}

/// [`Available`]'s discriminator.
///
/// One variant, and the reason [`ImageCheckResponse`] can be untagged:
/// [`Unavailable`] cannot produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum AvailableType {
    #[serde(rename = "available")]
    #[default]
    Available,
}

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
/// One variant, and the reason [`ImageCheckResponse`] can be untagged:
/// [`Available`] cannot produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum UnavailableType {
    #[serde(rename = "unavailable")]
    #[default]
    Unavailable,
}
