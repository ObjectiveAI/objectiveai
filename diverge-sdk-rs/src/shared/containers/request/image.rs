//! What image a container is made from: a digest, and where its
//! bytes may be fetched.

use serde::{Deserialize, Serialize};

use super::Reference;

/// The image, by its manifest digest and the references under which
/// its bytes may be fetched — what
/// [`images::check`](crate::provider::endpoints::images::check::client::request::Frame)
/// asks about, so a check that came back available names an image a
/// run can ask for, with nothing to translate between them.
///
/// # The digest is the image
///
/// A digest cannot be repointed at different content, so an image
/// means the same bytes every time, wherever they come from. That is
/// the whole of what the wire asserts: the provider runs the image
/// the digest names and no other. The references are not the image:
/// the same bytes sit under different names on different registries,
/// and an id that hashes an image hashes it with none — see
/// [`hashed`](Self::hashed).
///
/// # Who supplies the bytes is the provider's
///
/// A provider looks in what it holds first, by digest alone. Failing
/// that it may ask any referenced registry its own policy lists, with
/// the credential it holds for it, and ignores a reference naming one
/// it does not; and it may take the image from the caller — asking on
/// the run scope whether the caller holds it and, when it does, for
/// its manifest and blobs, see [`oci`](crate::shared::containers::oci).
/// Which of those, in what order, is the provider's policy, and a
/// caller learns only whether the run happened. A caller need not
/// hold what it names; one that does answers so when asked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Image {
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What identifies the image: a runtime hashes what it pulls, so
    /// a source serving other bytes under it fails before anything
    /// runs.
    pub digest: String,
    /// Where the bytes may be fetched, each a registry and a path
    /// there: see [`Reference`]. Absent when there are none, and then
    /// the provider has only what it holds and what the caller holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
}

impl Image {
    /// The image as an id hashes it: the digest with no references,
    /// so that one image under any names hashes the same.
    pub fn hashed(&self) -> Image {
        Image {
            digest: self.digest.clone(),
            references: Vec::new(),
        }
    }
}
