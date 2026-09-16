//! The section itself.

use serde::{Deserialize, Serialize};

use super::{Podman, ServerImage};

/// The `containers` section: the runtime the provider runs containers
/// with, what it gives it, and the images the provider offers as its
/// own.
///
/// `podman` is required when the section is present; `server_images`
/// may be left out, and then there are none. The section as a whole
/// may be absent, and then it is [`Default`]: every field its own
/// default, which is how a provider bootstraps.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// The podman the provider runs containers with: where it pulls
    /// from, where it keeps its data, and how much of the host it may
    /// take.
    pub podman: Podman,
    /// The images a caller may name as `server` images, each a
    /// repository path and a digest. `images::check` answers from
    /// this list and nothing else, and a `server` deploy runs a
    /// listed pair and nothing else. Absent means none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub server_images: Vec<ServerImage>,
}
