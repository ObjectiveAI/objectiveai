//! One image a caller built.

use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;

/// An image the caller built on the provider, under the name it gave.
///
/// [`name`](Self::name) is what to call it, [`digest`](Self::digest)
/// what it is, [`bytes`](Self::bytes) how big it is, and
/// [`created`](Self::created) when the build ended. That is the whole
/// of what a listing says about one; the Containerfile it was built
/// from is a [`stat`](crate::provider::endpoints::images::stat) away.
///
/// # The digest is how a run names it
///
/// A [`Container`](crate::shared::containers::request::Container)
/// names its image by digest, and this is the digest to name: a run
/// by the identity that built the image, naming it, runs it from the
/// store, with no reference needed and nobody asked. The name is the
/// handle in THIS family — a stat and a delete name the image by it
/// — and nowhere else.
///
/// # Nothing here changes
///
/// An image is built once and never edited, so a listing tells an
/// image added and removed and nothing between. A caller that wants
/// a different image builds one under another name, or deletes and
/// builds again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// What to call this image, and how to ask for it: the name the
    /// [`build`](crate::provider::endpoints::images::build) gave it,
    /// unique among the caller's images.
    pub name: String,
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: what a container request names.
    pub digest: String,
    /// Its size in BYTES, as the provider counts it against the
    /// store's capacity.
    pub bytes: u64,
    /// When the build ended, in SECONDS since the Unix epoch.
    pub created: u64,
}

/// Keyed by the name: an image whose digest differs is removed and
/// added, under the same key.
impl Keyed for Image {
    type Key = String;

    fn key(&self) -> String {
        self.name.clone()
    }
}
