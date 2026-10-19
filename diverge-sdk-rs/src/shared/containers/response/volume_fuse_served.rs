//! A run refused because a FUSE serve has a volume it names, on a
//! provider that does not mount a volume of that mode beside one.

use serde::{Deserialize, Serialize};

/// The volume a run request named that a
/// [`fuse_serve`](crate::provider::endpoints::volumes::fuse_serve)
/// holds at the time, on a provider whose `mount_and_fuse_serve` for
/// the volume's mode is `false` — see
/// [`VolumeCapabilities`](crate::provider::endpoints::info::server::response::VolumeCapabilities)
/// — and so refused the run.
///
/// The first and only response of its scope, then the finish, with
/// nothing fetched and nothing deployed. The volume and the FUSE serve
/// that has it are as they were; a caller stops that FUSE serve, names
/// another volume, or asks another provider, and asks again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeFuseServed {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
}
