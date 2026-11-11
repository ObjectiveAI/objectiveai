//! The graph written: an occurrence at a seam, recorded.

use diverge_sdk::daemon::creator::Creator;

use crate::daemon::{Daemon, Kind};
use crate::store::events;

/// Record that something passed from `sender` to `receiver`: the edge
/// added, or its count raised, and every events list kept open told.
///
/// The graph is a record of what happened and never a gate on it, so
/// a store that could not take the row — no connection to be had, the
/// insert refused — is nothing to the exchange recorded: the
/// occurrence goes uncounted, the exchange goes on, and the word is
/// not said, since nothing changed. So too an end whose row is gone,
/// deleted before the occurrence was recorded: an edge goes with its
/// parties. Nothing is reported, since the daemon prints nothing.
pub async fn record(daemon: &Daemon, sender: &Creator, receiver: &Creator) {
    let Ok(mut conn) = daemon.store.acquire().await else {
        return;
    };
    if !matches!(events::record(&mut conn, sender, receiver).await, Ok(true)) {
        return;
    }
    daemon.live.changed(Kind::Events);
}
