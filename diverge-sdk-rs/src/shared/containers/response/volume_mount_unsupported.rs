//! A run refused because the provider does not mount a volume of the
//! mode a volume it names is in.

use serde::{Deserialize, Serialize};

/// The volume a run request named whose
/// [`Mode`](crate::provider::endpoints::volumes::Mode) the provider
/// does not mount into containers at all: its `mount` for the mode is
/// `false` — see
/// [`VolumeCapabilities`](crate::provider::endpoints::info::server::response::VolumeCapabilities)
/// — and so refused the run.
///
/// The first and only response of its scope, then the finish, with
/// nothing fetched and nothing deployed. Nothing about the volume is
/// the cause: the same volume in a mode the provider mounts, after an
/// [`edit`](crate::provider::endpoints::volumes::edit), or another
/// provider, is what a caller asks next. A volume in a mode the
/// provider has no capability for at all is never listed, so a
/// request cannot name one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeMountUnsupported {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
}
