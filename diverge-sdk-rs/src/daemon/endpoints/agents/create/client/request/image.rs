//! What image an agent is made from: a digest, and where its bytes
//! may be fetched.

use serde::{Deserialize, Serialize};

use super::Reference;

/// The image, by its manifest digest and the references under which
/// its bytes may be fetched.
///
/// # The digest is the image
///
/// A digest cannot be repointed at different content, so an image
/// means the same bytes every time, wherever they come from. That is
/// the whole of what the request asserts: the agent runs the image
/// the digest names and no other. The references are not the image:
/// the same bytes sit under different names on different registries,
/// and a template's id hashes the image with none — see
/// [`hashed`](Self::hashed).
///
/// # Who supplies the bytes is not the caller's
///
/// The daemon asks a provider for the image, references and all, and
/// the provider holds it already, pulls it from a referenced registry
/// its policy lists, or takes it from the daemon; which, and from
/// where, is between them, and a caller learns only whether the agent
/// runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// The manifest digest, `<algorithm>:<hex>`.
    ///
    /// What identifies the image: a runtime hashes what it pulls, so
    /// a source serving other bytes under it fails before anything
    /// runs.
    pub digest: String,
    /// Where the bytes may be fetched, each a registry and a path
    /// there: see [`Reference`]. Absent when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Reference>,
}

impl Image {
    /// The image as a template's id hashes it: the digest with no
    /// references, so that one image under any names hashes the same.
    pub fn hashed(&self) -> Image {
        Image {
            digest: self.digest.clone(),
            references: Vec::new(),
        }
    }
}
