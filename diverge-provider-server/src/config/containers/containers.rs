//! The section itself.

use serde::{Deserialize, Serialize};

use super::{Identity, Podman};

/// The `containers` section: the runtime the provider runs containers
/// with and what it gives it, and the store of content mounted by
/// identity.
///
/// Every field is required when the section is present. The section
/// as a whole may be absent, and then it is [`Default`]: every field
/// its own default, which is how a provider bootstraps.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// The podman the provider runs containers with: where it pulls
    /// from, where it keeps its data, and how much of the host it may
    /// take.
    pub podman: Podman,
    /// The store of content mounted by identity: where it is kept,
    /// and how much of it there may be.
    pub identity: Identity,
}
