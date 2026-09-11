//! The caller's vault: every secret the run needs, and the cycle for
//! the ones it rotates.
//!
//! A secret is a vault key named by the setting it fills. The keys a
//! run needs are read off the agent by [`keys`]: every plugin's
//! `secrets`, on either list, and nothing the harness implies of its
//! own. A STATIC secret is [`statics`]: one `get`, no lock, no
//! set; a key the vault does not hold refuses the run, naming it. Two
//! secrets the harness needs of its own — Eliza's vault passphrase
//! and the settings salt — are [`minted`] when absent, so a lineage
//! never changes them and never has to provision them.
//!
//! A ROTATING secret owes Hermes's cycle: [`acquire`] locks the key
//! for the run, refreshed on a task before its TTL runs out, and reads
//! it; the run reads the rotated value back after every turn and
//! [`Held::set_if_changed`] sets it the moment it differs, so a
//! container that dies mid-run has already put the last rotation
//! where the caller's next run finds it; [`Held::release`] unlocks
//! every key at the end. A container that dies loses the locks within
//! the TTL.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use diverge_container_proxy_sdk::Client;
use tokio::task::JoinHandle;

use crate::agent::Agent;
use crate::agent::plugin::{Rotates, Secret};

/// How long each lock is granted for, in seconds.
pub const TTL: u32 = 300;

/// How often the held locks are refreshed — well inside the TTL.
pub const REFRESH: Duration = Duration::from_secs(100);

/// The passphrase Eliza's own vault derives its master key from:
/// minted here when absent, and the process environment's after.
pub const PASSPHRASE: &str = "ELIZA_VAULT_PASSPHRASE";

/// The salt Eliza encrypts settings with. Stable for the lineage:
/// `initialize()` merges the `agents` row's settings back into the
/// character, and a decrypt under a different salt throws.
pub const SALT: &str = "SECRET_SALT";

/// The keys a run needs, by cycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Keys {
    /// Read once, never set.
    pub statics: Vec<String>,
    /// Locked, read, read back and set, unlocked.
    pub rotating: Vec<Rotating>,
}

/// One rotating secret: its key, and where the rotated value lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotating {
    /// The setting name, and the vault key.
    pub key: String,
    /// Where the plugin leaves the rotated value.
    pub rotates: Rotates,
}

/// The keys the agent names, deduplicated in order: the model
/// providers' first, then the other plugins'.
pub fn keys(agent: &Agent) -> Keys {
    let mut keys = Keys::default();
    let mut push_static = |key: &str| {
        if !keys.statics.iter().any(|held| held == key) {
            keys.statics.push(key.to_string());
        }
    };
    for plugin in agent.model_provider_plugins.iter().chain(&agent.plugins) {
        for secret in &plugin.secrets {
            match secret {
                Secret::Static(key) => push_static(key),
                Secret::Rotating {
                    key,
                    rotates: Rotates::InPlace(false),
                } => push_static(key),
                Secret::Rotating { key, rotates } => {
                    if !keys.rotating.iter().any(|held| &held.key == key) {
                        keys.rotating.push(Rotating {
                            key: key.clone(),
                            rotates: rotates.clone(),
                        });
                    }
                }
            }
        }
    }
    keys
}

/// Read every static secret. A key the vault does not hold, or holds
/// as something other than text, fails the whole read.
pub async fn statics(
    client: &Client,
    keys: &[String],
) -> Result<BTreeMap<String, String>, Error> {
    let mut values = BTreeMap::new();
    for key in keys {
        values.insert(key.clone(), get(client, key).await?);
    }
    Ok(values)
}

/// A secret the harness owns: the vault's, or — absent — minted (two
/// v4 uuids, 64 hex characters) and set, so every later run reads the
/// same one.
pub async fn minted(client: &Client, key: &str) -> Result<String, Error> {
    let bytes = client
        .vault_get(key)
        .await
        .map_err(|error| Error::Get {
            key: key.to_string(),
            error,
        })?;
    if let Some(bytes) = bytes {
        return String::from_utf8(bytes.to_vec()).map_err(|_| Error::NotText(key.to_string()));
    }
    let value = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    client
        .vault_set(key, value.as_bytes())
        .await
        .map_err(|error| Error::Set {
            key: key.to_string(),
            error,
        })?;
    Ok(value)
}

/// Lock every rotating key and read its value, in order.
///
/// A key that cannot be locked, cannot be read, holds nothing, or
/// holds something other than text fails the whole acquisition;
/// whatever was locked by then is unlocked again, best effort, before
/// the error goes back. On success the refresh task starts,
/// re-locking every key every [`REFRESH`].
pub async fn acquire(
    client: &Arc<Client>,
    rotating: &[Rotating],
) -> Result<(BTreeMap<String, String>, Held), Error> {
    let mut locked: Vec<String> = Vec::new();
    let mut values = BTreeMap::new();
    let mut last = BTreeMap::new();
    for secret in rotating {
        let key = &secret.key;
        let result = async {
            client
                .vault_lock(key, TTL)
                .await
                .map_err(|error| Error::Lock {
                    key: key.clone(),
                    error,
                })?;
            locked.push(key.clone());
            get(client, key).await
        }
        .await;
        match result {
            Ok(value) => {
                last.insert(key.clone(), value.clone().into_bytes());
                values.insert(key.clone(), value);
            }
            Err(error) => {
                for key in locked {
                    let _ = client.vault_unlock(&key).await;
                }
                return Err(error);
            }
        }
    }
    let refresh = tokio::spawn({
        let client = Arc::clone(client);
        let keys = locked.clone();
        async move {
            loop {
                tokio::time::sleep(REFRESH).await;
                for key in &keys {
                    // A refresh that fails is not this task's to
                    // report: the lock lapses by its TTL, and the
                    // release at the run's end reports what it finds.
                    let _ = client.vault_lock(key, TTL).await;
                }
            }
        }
    });
    Ok((
        values,
        Held {
            client: Arc::clone(client),
            keys: locked,
            last,
            refresh,
        },
    ))
}

/// One key's value, as text.
async fn get(client: &Client, key: &str) -> Result<String, Error> {
    let bytes = client
        .vault_get(key)
        .await
        .map_err(|error| Error::Get {
            key: key.to_string(),
            error,
        })?
        .ok_or_else(|| Error::Missing(key.to_string()))?;
    String::from_utf8(bytes.to_vec()).map_err(|_| Error::NotText(key.to_string()))
}

/// The keys a run holds, the last value set under each, and the task
/// keeping them held.
#[must_use = "dropping the hold stops the refresh; the locks then lapse by TTL"]
pub struct Held {
    client: Arc<Client>,
    keys: Vec<String>,
    last: BTreeMap<String, Vec<u8>>,
    refresh: JoinHandle<()>,
}

impl Held {
    /// Set the value read back under a held key, if it differs from
    /// the last one set (or read). Answers whether it was set. A key
    /// not held is nothing to do.
    pub async fn set_if_changed(&mut self, key: &str, value: Vec<u8>) -> Result<bool, Error> {
        if !self.keys.iter().any(|held| held == key) {
            return Ok(false);
        }
        if self.last.get(key) == Some(&value) {
            return Ok(false);
        }
        self.client
            .vault_set(key, &value)
            .await
            .map_err(|error| Error::Set {
                key: key.to_string(),
                error,
            })?;
        self.last.insert(key.to_string(), value);
        Ok(true)
    }

    /// Unlock every key. Every key is attempted; the first failure is
    /// the one reported.
    pub async fn release(self) -> Result<(), Error> {
        self.refresh.abort();
        let mut first: Option<Error> = None;
        for key in &self.keys {
            if let Err(error) = self.client.vault_unlock(key).await {
                first.get_or_insert(Error::Unlock {
                    key: key.clone(),
                    error,
                });
            }
        }
        match first {
            None => Ok(()),
            Some(error) => Err(error),
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // A hold dropped without release — the run abandoned — stops
        // refreshing; the locks lapse by their TTL.
        self.refresh.abort();
    }
}

/// The vault could not give or take what the run needed.
#[derive(Debug)]
pub enum Error {
    /// The key could not be locked.
    Lock {
        key: String,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key could not be read.
    Get {
        key: String,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The vault holds nothing under the key.
    Missing(String),
    /// The key holds something that is not text, which every setting
    /// must be.
    NotText(String),
    /// The value could not be set.
    Set {
        key: String,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key could not be unlocked.
    Unlock {
        key: String,
        error: diverge_container_proxy_sdk::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Lock { key, error } => {
                write!(f, "the vault key {key} could not be locked: {error}")
            }
            Error::Get { key, error } => {
                write!(f, "the vault key {key} could not be read: {error}")
            }
            Error::Missing(key) => write!(f, "the vault holds no {key}"),
            Error::NotText(key) => write!(f, "the vault's {key} is not text"),
            Error::Set { key, error } => {
                write!(f, "the vault key {key} could not be set: {error}")
            }
            Error::Unlock { key, error } => {
                write!(f, "the vault key {key} could not be unlocked: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Lock { error, .. }
            | Error::Get { error, .. }
            | Error::Set { error, .. }
            | Error::Unlock { error, .. } => Some(error),
            Error::Missing(_) | Error::NotText(_) => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "vault",
            "error": self.to_string(),
        })
    }
}
