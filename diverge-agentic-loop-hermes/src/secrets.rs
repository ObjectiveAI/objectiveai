//! The secrets, as the proxy feeds and collects them.
//!
//! The program says which secrets it takes once, in its answer to
//! `POST /register` — the plan's
//! [`declaration`](crate::filesystem::Plan::declaration), each name
//! to its mode. From then on the values travel two ways. The proxy
//! FEEDS them on `POST /secrets`: once right after the registration
//! and again at every activation, and it may do so at any time — a
//! feed between loops is the next loop's values, a feed during a
//! loop is held and applied at the next one, since a loop takes its
//! [`values`] snapshot once at its start and keeps it. The proxy
//! COLLECTS the rotating ones on `POST /secrets/collect` at every
//! inactive, which for this program is right after a loop has
//! stopped the gateway and read its documents back
//! ([`crate::filesystem::read_back`]) — the loop [`leave`]s what it
//! read, and the collect answers from that.
//!
//! Two holdings, both the latest only. `VALUES` is the last feed,
//! whole. `LAST` is what the last loop read back, and a feed CLEARS
//! it: a collect that comes after a loop which failed before its
//! read-back answers the value fed for THIS activation, never a
//! previous loop's. A collect that finds a name in neither is the
//! program failing to give a value back, and says which name —
//! never a value.

use diverge_sdk::shared::containers::secrets::Values;
use tokio::sync::RwLock;

/// The last feed, whole. `None` until the first.
static VALUES: RwLock<Option<Values>> = RwLock::const_new(None);

/// What the last loop read back, by name. `None` until a loop has,
/// and again after every feed.
static LAST: RwLock<Option<Values>> = RwLock::const_new(None);

/// Hold the fed values, whole, in place of the previous feed, and
/// forget what the last loop read back.
pub async fn feed(values: Values) {
    let mut held = VALUES.write().await;
    let mut last = LAST.write().await;
    *held = Some(values);
    *last = None;
}

/// The fed values, as a snapshot: the loop takes one at its start
/// and keeps it, whatever is fed meanwhile. Empty before any feed —
/// a declared name then goes missing at the preparation, which is
/// the loop's refusal.
pub async fn values() -> Values {
    VALUES.read().await.clone().unwrap_or_default()
}

/// Hold what the loop read back — every rotating name as the run
/// left it — for the collect that follows.
pub async fn leave(values: Values) {
    *LAST.write().await = Some(values);
}

/// Answer a collect: each name from what the last loop read back,
/// else from the last feed. A name in neither is the error, and the
/// error is the name.
pub async fn collect(names: &[String]) -> Result<Values, String> {
    let held = VALUES.read().await;
    let last = LAST.read().await;
    let mut values = Values::new();
    for name in names {
        let value = last
            .as_ref()
            .and_then(|last| last.get(name))
            .or_else(|| held.as_ref().and_then(|held| held.get(name)));
        match value {
            Some(value) => {
                values.insert(name.clone(), value.clone());
            }
            None => return Err(name.clone()),
        }
    }
    Ok(values)
}
