//! A run refused because a running container has a volume it names,
//! on a provider that mounts a volume of that mode once.

use serde::{Deserialize, Serialize};

/// The volume a run request named that a running container has
/// mounted at the time, on a provider whose `mount_many` for the
/// volume's mode is `false` — see
/// [`VolumeCapabilities`](crate::provider::endpoints::info::server::response::VolumeCapabilities)
/// — and so refused the run.
///
/// The first and only response of its scope, then the finish, with
/// nothing fetched and nothing deployed. The volume and the container
/// that has it are as they were; a caller stops that container, names
/// another volume, or asks another provider, and asks again. A
/// persistent volume is the usual case, its
/// [`Mode`](crate::provider::endpoints::volumes::Mode) allowing one
/// user at a time; an ephemeral or a read-only one is this only where
/// the provider's capability for the mode says so.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeMounted {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
}
