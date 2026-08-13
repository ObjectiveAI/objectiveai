//! What a client's request frame carries for an image check.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

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
pub struct Frame {
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

impl Encode for Frame {
    /// The ordinary JSON failure. Two strings and nothing else — this
    /// is as close to unfailable as a serialized shape gets, but it is
    /// a shape, so it is not [`Infallible`](std::convert::Infallible).
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
