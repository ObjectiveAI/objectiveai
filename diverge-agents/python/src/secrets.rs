//! The secrets' values, as the proxy feeds them, held for the loop.
//!
//! The program declares which secrets it takes in its answer to
//! `POST /register` — this image's [`Agent::declaration`] — and the
//! proxy feeds their values on `POST /secrets`: once before the
//! container's id goes out, and again at every activation after,
//! which may be while a loop is in flight. Every feed replaces the
//! whole set. Nothing is read from the proxy: the values come to the
//! program, and the program holds them here, in memory, for the
//! container's life.
//!
//! This image declares NOTHING: no credential exists — there is no
//! model to call — and the script is handed nothing of what is held
//! here, so no run reads it. The two calls are served all the same,
//! as the contract — a feed is held, and `POST /secrets/collect`,
//! which asks the `rotating` ones back at every inactive and never
//! comes for a program that declared none, answers every name asked
//! with the value it was fed. A Python agent that wants a secret
//! would declare it here and add it to the script's stdin feed; not
//! now.
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
