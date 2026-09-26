//! A run refused because a volume it names is held to itself by a
//! stat, an edit or a delete.

use serde::{Deserialize, Serialize};

/// The volume a run request named that is under a stat, an edit or
/// a delete of the same caller at the time, and so refused the run.
///
/// A volume may be mounted in any number of containers of its caller
/// at once, whatever its `persist`; what it cannot be is
/// mounted while something examines, resizes or removes it. A
/// provider holds every volume a run names, shared, from the moment
/// it accepts the request until the run ends, and a request naming
/// one that is held exclusively is answered with this — the first
/// and only response of its scope, then the finish — with nothing
/// fetched and nothing deployed for it. On the server half the hold
/// is the volume's [`mount`](crate::server::volume::Volume::mount),
/// which the run handler takes before anything else and gives back
/// on every ending.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeHeld {
    /// The `volume_name` of the volume, as the request named it.
    pub name: String,
}
