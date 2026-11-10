//! The secrets' values, as the proxy feeds them, held for the loop.
//!
//! The program declares which secrets it takes in its answer to
//! `POST /register` — this image's [`Agent::declaration`] — and the
//! proxy feeds their values on `POST /secrets`: once before the
//! container's id goes out, and again at every activation after,
//! which may be while a loop is in flight. Every feed replaces the
//! whole set; a loop takes its snapshot once, at its start, and keeps
//! what it took, so a feed mid-loop lands on the next loop. Nothing
//! is read from the proxy: the values come to the program, and the
//! program holds them here, in memory, for the container's life.
//!
//! `POST /secrets/collect` asks the `rotating` ones back at every
//! inactive. This image declares none — its one secret is `fixed` —
//! so the ask never comes; the answer is served all the same, as the
//! contract: every name asked, with the value it was fed, since a
//! value the program never changed is the value it leaves.
//!
//! No value is ever written into an error or a log: a failure names
//! the secret, and nothing else.
//!
//! [`Agent::declaration`]: crate::agent::Agent::declaration

use diverge_sdk::shared::containers::secrets::Values;
use tokio::sync::RwLock;

/// The values, as the last feed left them: none before the first,
/// since nothing can be fed before the registration is answered.
static VALUES: RwLock<Option<Values>> = RwLock::const_new(None);

/// Hold the values a feed carries, replacing whatever the last feed
/// left — the whole set, so a name the new feed lacks is gone.
pub async fn feed(values: Values) {
    *VALUES.write().await = Some(values);
}

/// The values as last fed, a snapshot the loop takes once at its
/// start. `None` is no feed yet: the registration was answered, but
/// the proxy has not fed the program — a run that early is refused.
pub async fn values() -> Option<Values> {
    VALUES.read().await.clone()
}

/// Each name asked, with the value it was fed. `Err` names the first
/// name the program holds no value for — never fed, or fed under
/// another set since — and never a value.
pub async fn collect(names: &[String]) -> Result<Values, String> {
    let held = VALUES.read().await;
    let mut collected = Values::with_capacity(names.len());
    for name in names {
        match held.as_ref().and_then(|values| values.get(name)) {
            Some(value) => {
                collected.insert(name.clone(), value.clone());
            }
            None => {
                return Err(format!("the program holds no value for the secret `{name}`"));
            }
        }
    }
    Ok(collected)
}
