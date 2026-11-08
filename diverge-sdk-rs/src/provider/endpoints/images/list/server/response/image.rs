//! One image in the caller's Store.

use serde::{Deserialize, Serialize};

use crate::shared::discover::Description;
use crate::shared::listing::Keyed;

/// An image the caller owns in its Store on the provider: one it
/// built or pushed there, by its digest.
///
/// [`digest`](Self::digest) is what it is, and the only way it is
/// named; [`description`](Self::description) what the caller said of
/// it; [`bytes`](Self::bytes) how big it is; and
/// [`created`](Self::created) when the caller came to own it. That is
/// the whole of what a listing says about one; the Containerfile a
/// built one was built from is a
/// [`stat`](crate::provider::endpoints::images::stat) away.
///
/// # The digest is the image, and its only name
///
/// A [`Container`](crate::shared::containers::request::Container)
/// names its image by digest, and this is the digest to name: a run
/// by the identity that owns the image, naming it, runs it from the
/// Store, with no reference needed and nobody asked. A stat, an edit,
/// a hash and a delete name it by the same digest. There is no other
/// name: one image is one digest, owned once per identity.
///
/// # Only the description changes
///
/// The bytes under a digest never change, so a listing tells an image
/// added and removed — and [`Changed`](super::Frame::Changed) when the
/// caller edits its description, and for nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: what a container request names, and the
    /// key of the listing.
    pub digest: String,
    /// What the caller said of the image, at most 132 characters, or
    /// `None` for nothing said. Set by the
    /// [`build`](crate::provider::endpoints::images::build) or the
    /// [`push`](crate::provider::endpoints::images::push) that gained
    /// it, and by an [`edit`](crate::provider::endpoints::images::edit).
    pub description: Option<Description>,
    /// Its size in BYTES: every blob it reaches and its manifests,
    /// as the provider counts them.
    pub bytes: u64,
    /// When the caller came to own it — the build or the push ended —
    /// in SECONDS since the Unix epoch.
    pub created: u64,
}

/// Keyed by the digest: one image is one digest in one caller's
/// Store.
impl Keyed for Image {
    type Key = String;

    fn key(&self) -> String {
        self.digest.clone()
    }
}
