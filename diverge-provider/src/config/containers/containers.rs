//! The section itself.

use serde::{Deserialize, Serialize};

use super::Podman;

/// The `containers` section: the runtime the provider runs containers
/// with, and what it gives it.
///
/// `podman` is required when the section is present; `enabled` is
/// not, and is written only when `false`. The section as a whole may
/// be absent, and then it is [`Default`]: every field its own
/// default, which is how a provider bootstraps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// Whether the provider runs containers at all. `false` refuses
    /// every agents run and tools run with `Unsupported`, before
    /// anything is held, fetched or deployed, and is the `containers`
    /// capability `false` in the provider's `info`. Absent means
    /// `true`, so a provider that never wrote it runs containers.
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub enabled: bool,
    /// The podman the provider runs containers with: where it pulls
    /// from, where it keeps its data, and how much of the host it may
    /// take.
    pub podman: Podman,
}

/// What a provider runs with before it has written a line of
/// configuration: containers run, with the [`Podman`] defaults.
impl Default for Containers {
    fn default() -> Self {
        Containers {
            enabled: enabled(),
            podman: Podman::default(),
        }
    }
}

/// The value `enabled` takes when the file leaves it out.
fn enabled() -> bool {
    true
}

/// Whether `enabled` is its default, and so left out of the file.
fn is_enabled(enabled: &bool) -> bool {
    *enabled
}
