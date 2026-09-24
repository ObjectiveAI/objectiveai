//! What image an agent is made from: a name and a digest.

use serde::{Deserialize, Serialize};

/// The image, by its repository path and its manifest digest.
///
/// # The digest is the image
///
/// A digest cannot be repointed at different content, so an image
/// means the same bytes every time, wherever they come from. That is
/// the whole of what the request asserts: the agent runs the image
/// the digest names and no other.
///
/// # Who supplies the bytes is not the caller's
///
/// Not on the wire. The daemon asks a provider for the image, and the
/// provider holds it already, pulls it, or takes it from the daemon;
/// which, and from where, is between them, and a caller learns only
/// whether the agent runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// The repository path — `library/nginx`, `myorg/myimage`. No
    /// host: where the bytes come from is not the caller's to say.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image: a runtime hashes what it
    /// pulls, so a source serving other bytes under it fails before
    /// anything runs.
    pub digest: String,
}
