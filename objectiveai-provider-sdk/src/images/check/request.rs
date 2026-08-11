//! The image check request.

use serde::{Deserialize, Serialize};

/// Ask a provider whether it can supply a particular image.
///
/// No registry. The digest is the image's identity and a registry is
/// only a locator — any registry serving these bytes serves the same
/// image, because a client recomputes the hash on pull and rejects a
/// mismatch. So WHERE a provider gets it is the provider's business:
/// its own mirror, a pull-through cache, a private registry it has
/// credentials for, or something it already holds locally.
///
/// That is also what makes proprietary images answerable. A provider
/// with access to an image no public registry serves can still say
/// yes, and a caller naming a registry it cannot reach would be
/// asserting something it has no standing to assert.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ImageCheckRequest {
    /// The repository path — `library/nginx`, `myorg/myimage`.
    ///
    /// Kept alongside the digest because a digest alone is not
    /// resolvable: every registry API is repository-scoped, and there
    /// is no lookup from a digest to wherever it lives.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image. Unlike a tag, it cannot be
    /// repointed at different content.
    pub digest: String,
}
