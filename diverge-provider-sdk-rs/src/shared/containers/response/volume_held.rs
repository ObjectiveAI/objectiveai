//! A run refused because a volume it names is held: to itself by a
//! stat, an edit or a delete, or, a persistent one, by anyone.

use serde::{Deserialize, Serialize};

/// The volume a run request named that is held at the time — under
/// a stat, an edit or a delete of the same caller, or, a persistent
/// volume, mounted in a running container or served — and so refused
/// the run.
///
/// An ephemeral or a read-only volume may be mounted in any number of
/// containers of its caller at once; a persistent one has one user at
/// a time, one running container or one serve, as its
/// [`Mode`](crate::endpoints::volumes::Mode) states. No volume is
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
