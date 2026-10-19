//! The graph written: an occurrence at a seam, recorded.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::events::Action;

use crate::daemon::{Daemon, Kind};
use crate::store::events;

/// Record that `action` passed from `sender` to `receiver`: the edge
/// added, or its count raised, and every events list kept open told.
///
/// The graph is a record of what happened and never a gate on it, so
/// a store that could not take the row — no connection to be had, the
/// insert refused — is nothing to the exchange recorded: the
/// occurrence goes uncounted, the exchange goes on, and the word is
/// not said, since nothing changed. Nothing is reported, since the
/// daemon prints nothing.
pub async fn record(daemon: &Daemon, sender: &Creator, receiver: &Creator, action: Action) {
    let Ok(mut conn) = daemon.store.acquire().await else {
        return;
    };
    if events::record(&mut conn, sender, receiver, action).await.is_err() {
        return;
    }
    daemon.live.changed(Kind::Events);
}
