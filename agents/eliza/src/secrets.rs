//! The secrets: declared at registration, fed before every loop,
//! collected after.
//!
//! A secret is a setting named by a plugin's entry, declared to the
//! proxy by name and mode in the registration's answer — this image's
//! [`Agent::declaration`] — and valued by the caller: the proxy feeds
//! the values on `POST /secrets`, once before the container's id goes
//! out and again at every activation after, which may be while a run
//! is in flight. Every feed replaces the whole set; a run takes its
//! snapshot once, at its start, and keeps what it took, so a feed
//! mid-run lands on the next run. Nothing is read from the proxy: the
//! values come to the program, and the program holds them here, in
//! memory, for the container's life. `Values` is JSON and a setting
//! is text, so a fed value reaches Eliza rendered through
//! [`settings::text`], and a name fed `null` is as much a refusal as
//! a name not fed at all.
//!
//! `POST /secrets/collect` asks the `rotating` ones back at every
//! inactive — the run's end — and every name asked must be answered.
//! The run reads a rotating value back from where its plugin leaves
//! it after every turn and [`leave`]s it here the moment it differs
//! from the value held, so a container that dies mid-run has already
//! put the last rotation where the collect finds it; a name nothing
//! was read back for is answered with the value it was fed, since a
//! value the plugin never changed is the value it leaves. A feed
//! clears what the last run left, so a collect after a run that
//! failed before its read-back answers the value fed for THIS
//! activation, never the previous run's.
//!
//! No value is ever written into an error, a notification or a log:
//! a failure names the secret, and nothing else.
//!
//! [`Agent::declaration`]: crate::agent::Agent::declaration

use std::sync::OnceLock;

use diverge_sdk::shared::containers::secrets::{Declaration, Values};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::settings;

/// The declaration, as the registration answered it: the names the
/// agent's plugins read, each to its mode. None before registration.
static DECLARED: OnceLock<Declaration> = OnceLock::new();

/// The values, as the last feed left them: none before the first,
/// since nothing can be fed before the registration is answered.
static VALUES: RwLock<Option<Values>> = RwLock::const_new(None);

/// What the running or last run left: seeded with the snapshot a run
/// starts on, then each rotating name's value as the run read it
/// back. None before the first run, and again after every feed.
static LAST: RwLock<Option<Values>> = RwLock::const_new(None);

/// Hold the declaration the registration answered with. Once: the
/// registration fixes the agent for the container's life, and the
/// declaration is the agent's, so a later call keeps the first.
pub fn declare(declaration: Declaration) {
    DECLARED.get_or_init(|| declaration);
}

/// The declaration, if the agent is registered: what every feed is
/// checked against.
pub fn declared() -> Option<&'static Declaration> {
    DECLARED.get()
}

/// Hold the values a feed carries, replacing whatever the last feed
/// left — the whole set, so a name the new feed lacks is gone — and
/// forgetting what the last run left, so a collect from here on
/// answers this feed until a run reads something back.
pub async fn feed(values: Values) {
    *VALUES.write().await = Some(values);
    *LAST.write().await = None;
}

/// The declared secrets with their fed values, in the order declared:
/// the snapshot a run takes once at its start. A declared name the
/// last feed lacks, or carries as `null`, is the run's refusal,
/// naming it. Nothing declared is nothing owed — an agent with no
/// secrets is never fed, and runs.
pub async fn values() -> Result<Values, Error> {
    let held = VALUES.read().await;
    let mut values = Values::new();
    for name in declared().into_iter().flat_map(|declared| declared.keys()) {
        match held.as_ref().and_then(|fed| fed.get(name)) {
            None => return Err(Error::Missing(name.clone())),
            Some(Value::Null) => return Err(Error::Null(name.clone())),
            Some(value) => {
                values.insert(name.clone(), value.clone());
            }
        }
    }
    Ok(values)
}

/// Seed what the run leaves with the snapshot it starts on, so a
/// collect before any read-back — and after one that found nothing
/// — answers the value fed.
pub async fn seed(values: &Values) {
    *LAST.write().await = Some(values.clone());
}

/// Leave the value a run read back under a rotating name, as the
/// text the plugin left: stored when it differs from the value held
/// under the name rendered as a setting is, so a value the plugin
/// never changed stays the value it was fed, in the shape it was fed.
pub async fn leave(name: &str, read: String) {
    let mut last = LAST.write().await;
    let held = last.get_or_insert_with(Values::new);
    if held.get(name).and_then(settings::text).as_deref() == Some(read.as_str()) {
        return;
    }
    held.insert(name.to_string(), Value::String(read));
}

/// Each name asked, with the value the program leaves in it: what the
/// run last read back or started on, else what the last feed carried.
/// `Err` names the first name the program holds no value for — never
/// fed, or fed under another set since — and never a value.
pub async fn collect(names: &[String]) -> Result<Values, Error> {
    let last = LAST.read().await;
    let fed = VALUES.read().await;
    let mut collected = Values::with_capacity(names.len());
    for name in names {
        let value = last
            .as_ref()
            .and_then(|left| left.get(name))
            .or_else(|| fed.as_ref().and_then(|values| values.get(name)));
        match value {
            Some(value) => {
                collected.insert(name.clone(), value.clone());
            }
            None => return Err(Error::Missing(name.clone())),
        }
    }
    Ok(collected)
}

/// A secret the program holds no usable value for. Names the secret,
/// never the value.
#[derive(Debug)]
pub enum Error {
    /// The last feed carried no value under the name, or no feed has
    /// come at all.
    Missing(String),
    /// The last feed carried `null` under the name, and a setting
    /// must be text.
    Null(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(name) => {
                write!(f, "the program holds no value for the secret `{name}`")
            }
            Error::Null(name) => {
                write!(f, "the secret `{name}` was fed null, and a setting must be text")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "secrets",
            "error": self.to_string(),
        })
    }
}
