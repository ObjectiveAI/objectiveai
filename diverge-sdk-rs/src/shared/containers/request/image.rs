//! What image a container is made from: a digest, and where its
//! bytes may be fetched.

use serde::{Deserialize, Serialize};

use super::Reference;

/// The image, by its manifest digest and the references under which
/// its bytes may be fetched — what
/// [`images::read`](crate::provider::endpoints::images::read::client::request::Frame)
/// asks about, so an oci scope that came back held names an image a
/// run can ask for, with nothing to translate between them. The same
/// value is the whole of the shared
/// [`oci::has`](crate::shared::containers::oci::has) request a
/// provider asks a caller on a run, and the image every
/// [`oci::prove`](crate::shared::containers::oci::prove) ask on a
/// scope naming it is over.
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
/// # Where the bytes come from
///
/// A provider looks in the caller's own Store first, by digest alone —
/// the images the caller
/// [built](crate::provider::endpoints::images::build) or
/// [pushed](crate::provider::endpoints::images::push) on it, and no
/// other caller's. Failing that it asks any referenced registry its
/// own policy lists, with the credential it holds for it, and ignores
/// a reference naming one it does not; and failing that it takes the
/// image from the caller — asking on the run scope whether the caller
/// holds it, naming this image, and, when it does, for its manifest
/// and blobs, then for the proof that it holds it, see
/// [`oci`](crate::shared::containers::oci). The order, and which
/// answered, is stated on
/// [`Source`](crate::shared::containers::response::Source), which the
/// run's answer carries. A caller need not hold what it names; one
/// that does answers so when asked.
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
