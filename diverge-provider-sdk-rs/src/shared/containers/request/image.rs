//! What image a container is made from: a name and a digest.

use serde::{Deserialize, Serialize};

/// The image, by its repository path and its manifest digest — the
/// pair [`images::check`](crate::endpoints::images::check::client::request::Frame)
/// asks about, so a check that came back available names an image a
/// run can ask for, with nothing to translate between them.
///
/// # The digest is the image
///
/// A digest cannot be repointed at different content, so an image
/// means the same bytes every time, wherever they come from. That is
/// the whole of what the wire asserts: the provider runs the image
/// the digest names and no other.
///
/// # Who supplies the bytes is the provider's
///
/// Not on the wire. A provider may hold the image already, pull it
/// from a registry it uses, or take it from the caller — asking on
/// the run scope whether the caller holds it and, when it does, for
/// its manifest and blobs, see [`oci`](crate::shared::containers::oci).
/// Which of those, in what order, and which registries, is the
/// provider's policy, and a caller learns only whether the run
/// happened. A caller need not hold what it names; one that does
/// answers so when asked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// The repository path — `library/nginx`, `myorg/myimage`. No
    /// host: where the bytes come from is not the caller's to say.
    ///
    /// It lands in a reference by concatenation wherever a provider
    /// pulls from, so a provider refuses one that is not a repository
    /// path and normalizes nothing.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image: a runtime hashes what it
    /// pulls, so a source serving other bytes under it fails before
    /// anything runs.
    pub digest: String,
}
