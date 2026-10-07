//! Whether a container is served through a route.

use crate::daemon::Daemon;
use crate::store::routes::Route;

/// Whether an active container is served the route's tool now, which
/// refuses the route's deletion.
pub async fn in_use(daemon: &Daemon, route: &Route) -> bool {
    daemon.live.serving(route.tool).await
}
