//! A run refused because a volume it names is held to itself by an
//! operation: a stat, a read, a write, a filetree, an edit or a
//! delete.

use serde::{Deserialize, Serialize};

/// The volume a run request named that is held exclusively at the
/// time — under a stat, a read, a write, a filetree, an edit or a
/// delete of the same caller — and so refused the run.
///
/// No volume is mounted while something examines, reads, writes,
/// walks, resizes or removes it, whatever its
/// [`Mode`](crate::provider::endpoints::volumes::Mode). A provider
/// holds every volume a run names, shared, from the moment it accepts
/// the request until the run ends, and a request naming one that is
/// held exclusively is answered with this — the first and only
/// response of its scope, then the finish — with nothing fetched and
/// nothing deployed; a caller waits for the operation to end and asks
/// again. A volume another RUN or a FUSE serve has is not this: that
/// is [`VolumeMounted`](super::VolumeMounted) or
/// [`VolumeFuseServed`](super::VolumeFuseServed), each where the
/// provider's capability for the mode refuses a second holder. On the
/// server half the hold is the volume's `mount`, which the run
/// handler takes before anything else and gives back on every ending.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeHeld {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
}
