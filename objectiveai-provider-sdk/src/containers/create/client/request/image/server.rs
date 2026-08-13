//! The provider produces the image.

use serde::{Deserialize, Serialize};

/// The provider already has the image, or can get it its own way.
///
/// No registry, and that is the point. Where a provider gets an image
/// is the provider's business — its own mirror, a pull-through cache,
/// a private registry it holds credentials for, or something already
/// on disk. A caller naming a source it cannot reach would be
/// asserting something it has no standing to assert.
///
/// Which is what makes proprietary images expressible. A provider can
/// serve an image no public registry carries, and a caller can ask for
/// it, without the caller ever being able to fetch it itself.
///
/// Ask [`images::check`](crate::images::check) first if the answer
/// matters before the container does.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Server {
    /// The discriminator.
    pub r#type: ServerType,
    /// The repository path — `library/nginx`, `myorg/myimage`.
    ///
    /// Kept alongside the digest because a digest alone is not
    /// resolvable: every registry API is repository-scoped, and there
    /// is no lookup from a digest to wherever it lives.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image. Unlike a tag it cannot be
    /// repointed at different content, so a container created twice
    /// from this is created twice from the same bytes.
    pub digest: String,
}

/// [`Server`]'s discriminator.
///
/// One variant, and part of why [`Image`](super::Image) can be
/// untagged: no other source can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ServerType {
    /// Always this.
    #[serde(rename = "server")]
    #[default]
    Server,
}
