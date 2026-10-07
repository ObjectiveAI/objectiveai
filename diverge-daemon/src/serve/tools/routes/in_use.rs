//! Whether a container is served through a route.

use crate::daemon::Daemon;
use crate::store::routes::Route;

/// Whether an active container is served its dependency through the
/// route now, which refuses its deletion. No container runs yet.
pub fn in_use(_: &Daemon, _: &Route) -> bool {
    false
}
