//! One image in a Store of the daemon's, as its provider lists it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::agent_log::Identity;
use crate::shared::description::Description;
use crate::shared::listing::Keyed;

/// One image the daemon owns: the provider whose Store it is in, and
/// what that provider's listing says of it — its digest, what was said
/// of it, its size, and when the daemon came to own it. No creator and
/// no tags: an image is no record of the daemon's, and the Store on a
/// provider is the daemon's own, whichever account built or pushed
/// into it.
///
/// The digest is what a template's image names: a container the
/// daemon runs on that provider naming it runs it from the Store,
/// with nothing pulled and nobody asked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Image {
    /// The provider whose Store of the daemon's holds it: see
    /// [`Identity`].
    pub provider: Identity,
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: the whole of how it is named, with the
    /// provider.
    pub digest: String,
    /// What was said of the image by the build or the push that gained
    /// it: one short line. Absent, nothing was said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    /// Its size in BYTES: every blob it reaches and its manifests, as
    /// the provider counts them against the Store's capacity.
    pub bytes: u64,
    /// When the daemon came to own it — the build or the push ended —
    /// as the provider says: its listing's seconds since the Unix
    /// epoch, as an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
}

/// Keyed by the provider and the digest: one image is one digest,
/// owned once on a provider.
impl Keyed for Image {
    type Key = (Identity, String);

    fn key(&self) -> (Identity, String) {
        (self.provider.clone(), self.digest.clone())
    }
}
