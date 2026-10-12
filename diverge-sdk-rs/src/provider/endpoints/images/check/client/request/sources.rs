//! Where a check is to look.

use serde::{Deserialize, Serialize};

/// The places a provider may look for the image, each asked for or
/// not, either or both, and at least one asked for: a request naming
/// neither is answered
/// [`NoSource`](crate::provider::endpoints::images::check::server::response::Frame::NoSource)
/// and nothing is determined about the image.
///
/// # The two
///
/// [`client`](Self::client) is the caller's own images — those it
/// [built](crate::provider::endpoints::images::build) on this
/// provider and has not deleted — and no other identity's: a
/// provider never answers one identity's store to another.
/// [`registries`](Self::registries) is every referenced registry the
/// provider's own policy lists, asked as `<registry>/<name>@<digest>`;
/// a reference naming a registry not listed is ignored, and a
/// request with no reference asks nothing of this source.
///
/// # Never the provider's own
///
/// What the provider holds on its own account — its cache of images
/// pulled, whatever it loaded itself — is not a source and cannot be
/// asked for. A check answers nothing of it, so a check reveals
/// nothing a client could not pull for itself: an image the provider
/// pulled for one caller is not thereby available to another.
///
/// # Local first
///
/// Among the sources asked, the provider looks in the caller's own
/// images before it asks a registry, and a registry is asked only
/// when the caller's own images, where asked, did not have the
/// image. The answer is the same either way: available when any
/// source asked has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Sources {
    /// The caller's own images, built on this provider. Absent is
    /// `false`.
    #[serde(default)]
    pub client: bool,
    /// The referenced registries the provider's policy lists. Absent
    /// is `false`.
    #[serde(default)]
    pub registries: bool,
}

impl Sources {
    /// Whether any source is asked for: `false` is a request the
    /// provider answers `NoSource`.
    pub fn any(&self) -> bool {
        self.client || self.registries
    }
}
