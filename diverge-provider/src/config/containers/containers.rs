//! The section itself.

use serde::{Deserialize, Serialize};

use super::{Podman, ServerImage};

/// The `containers` section: the runtime the provider runs containers
/// with, what it gives it, and the images the provider holds itself.
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
    /// The images the provider holds itself, each a repository path
    /// and a digest: run from the store, never pulled, the first place
    /// a run and a check look. Absent means none, and every image is
    /// looked for in the registries and with the caller.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub server_images: Vec<ServerImage>,
}
