//! The secrets: fed by the proxy, held here, the rotating one handed
//! back.
//!
//! The agent declares, at its registration, the one name its login
//! method takes and the mode it takes it in; the proxy feeds the
//! values on `POST /secrets` — once after the registration, before
//! the container's id goes out, and again at every activation — and
//! collects the `rotating` ones on `POST /secrets/collect` at every
//! inactive. Nothing here dials anything: the values arrive, are held
//! whole, and a loop takes a snapshot of them once at its start, so a
//! feed that lands mid-loop is the next loop's and the loop in flight
//! keeps what it took.
//!
//! Two maps. [`VALUES`] is the last feed, replaced whole by every
//! feed. [`LAST`] is what the loop left in a rotating secret after it
//! read the value back — Codex's `auth.json` as it stands after the
//! last turn — and a collect answers from it first, then from the
//! feed, so that a value the loop could not read back is answered
//! with the value it was fed and the collect still carries every
//! name asked. A feed CLEARS [`LAST`]: a collect after a loop that
//! failed before its read-back answers the value fed for THIS
//! activation, never a previous loop's.

use diverge_sdk::shared::containers::secrets::Values;
use serde_json::Value;
use tokio::sync::RwLock;

/// The values as last fed: every name the agent declared, to its
/// value. `None` before the first feed — `IndexMap::new` is not
/// const, so the empty map is the option's.
static VALUES: RwLock<Option<Values>> = RwLock::const_new(None);

/// What the loop left in a rotating secret since the last feed, by
/// name; cleared by every feed.
static LAST: RwLock<Option<Values>> = RwLock::const_new(None);

/// Hold a feed: the values replaced whole, and what the previous
/// loop left forgotten. Both locks are taken in the order
/// [`collect`] takes them.
pub async fn feed(values: Values) {
    let mut last = LAST.write().await;
    let mut held = VALUES.write().await;
    *held = Some(values);
    *last = None;
}

/// A snapshot of the values as last fed, for a loop to take once at
/// its start. Empty before the first feed, so a declared name is
/// missing and the run's refusal says so.
pub async fn values() -> Values {
    VALUES.read().await.clone().unwrap_or_default()
}

/// What the loop leaves in a rotating secret: the value as it read it
/// back, answered to the next collect in place of the fed one.
pub async fn leave(name: &str, value: Value) {
    LAST.write()
        .await
        .get_or_insert_with(Values::new)
        .insert(name.to_string(), value);
}

/// The values to hand back: each name asked, from what the loop left
/// first, else from the feed, else the error naming the name — the
/// proxy needs every name it asked, and a name it was never fed has
/// nothing to be answered with.
pub async fn collect(names: &[String]) -> Result<Values, Error> {
    let last = LAST.read().await;
    let held = VALUES.read().await;
    let mut values = Values::new();
    for name in names {
        let left = last.as_ref().and_then(|last| last.get(name));
        let fed = held.as_ref().and_then(|held| held.get(name));
        match left.or(fed) {
            Some(value) => {
                values.insert(name.clone(), value.clone());
            }
            None => return Err(Error::Missing(name.clone())),
        }
    }
    Ok(values)
}

/// A collect could not be answered whole.
#[derive(Debug)]
pub enum Error {
    /// A name asked for that no feed carried and no loop left.
    Missing(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(name) => write!(f, "no value is held for the secret {name}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// The failure as JSON, the shape every failure on the stream
    /// has. Names the secret, never a value.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "secrets",
            "error": self.to_string(),
        })
    }
}
