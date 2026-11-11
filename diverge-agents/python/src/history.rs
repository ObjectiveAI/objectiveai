//! The continuation, cached: what the row holds, remembered.

use tokio::sync::Mutex;

use crate::continuation::Continuation;

/// What the database row held the last time this container looked or
/// wrote — or nothing, before it ever looked.
///
/// The outer `None` is "never loaded"; the inner is the row itself,
/// absent for a fresh conversation. Runs after the first start from
/// here instead of the row, and every successful save updates it, so
/// it never says more than the row does. A save that failed leaves it
/// as it was: the row is the truth, and the cache follows it.
static HISTORY: Mutex<Option<Option<Continuation>>> = Mutex::const_new(None);

/// The cached continuation, if this container has loaded one.
pub async fn cached() -> Option<Option<Continuation>> {
    HISTORY.lock().await.clone()
}

/// Remember what the row holds — after a load, or after a save that
/// succeeded.
pub async fn remember(history: Option<Continuation>) {
    *HISTORY.lock().await = Some(history);
}
