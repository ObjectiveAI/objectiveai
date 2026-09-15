//! A run refused because a volume it names is mounted elsewhere.

use serde::{Deserialize, Serialize};

/// The volume a run request named that is mounted in another
/// container of the same caller, and so refused the run.
///
/// A volume is mounted in at most one container of its caller at a
/// time, whatever the mount's `persist`. A provider holds every
/// volume a run names from the moment it accepts the request until
/// the run ends, and a second request naming one is answered with
/// this — the first and only response of its scope, then the finish
/// — with nothing fetched and nothing deployed for it. On the server
/// half the hold is the volume's
/// [`lock`](crate::server::volume::Volume::lock), which the run
/// handler takes before anything else and gives back on every
/// ending.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeMounted {
    /// The `host_name` of the volume, as the request named it.
    pub name: String,
}
