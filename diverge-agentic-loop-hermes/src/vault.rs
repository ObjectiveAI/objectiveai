//! The vault cycle for a rotating credential: lock, get, run, set,
//! unlock.
//!
//! A rotating OAuth document is mutated by the run — Hermes burns a
//! single-use refresh token and writes the replacement — so the copy
//! in the caller's vault must be the current one when the next run
//! reads it, wherever that run happens. The document lives in the
//! vault under a well-known key (the SDK's
//! [`vault::keys`](diverge_provider_sdk::shared::containers::vault::keys)),
//! and every run that needs it owes the cycle: [`acquire`] locks the
//! key and reads the document; the run writes it where Hermes reads
//! it and runs; [`Held::release`] sets the document as the run left
//! it and unlocks. The lock is held for the run, refreshed on a task
//! before its TTL runs out, so two runs never rotate one login past
//! each other; a container that dies loses the lock within the TTL.

use std::sync::Arc;
use std::time::Duration;

use diverge_container_proxy_sdk::Client;
use tokio::task::JoinHandle;

/// How long each lock is granted for, in seconds.
pub const TTL: u32 = 300;

/// How often the held locks are refreshed — well inside the TTL.
pub const REFRESH: Duration = Duration::from_secs(100);

/// A credential document: a JSON object, as every state document
/// here is.
pub type Document = serde_json::Map<String, serde_json::Value>;

/// The keys a run holds, and the task keeping them held.
#[must_use = "dropping the hold stops the refresh; the locks then lapse by TTL"]
pub struct Held {
    client: Arc<Client>,
    keys: Vec<&'static str>,
    refresh: JoinHandle<()>,
}

/// Lock every key and read its document, in order.
///
/// A key that cannot be locked, cannot be read, holds nothing
/// ([`Missing`](Error::Missing)), or holds something that is not a
/// JSON object fails the whole acquisition; whatever was locked by
/// then is unlocked again, best effort, before the error goes back.
/// On success the refresh task starts, re-locking every key every
/// [`REFRESH`].
pub async fn acquire(
    client: &Arc<Client>,
    keys: &[&'static str],
) -> Result<(Vec<(&'static str, Document)>, Held), Error> {
    let mut locked: Vec<&'static str> = Vec::new();
    let mut documents = Vec::new();
    for &key in keys {
        let result = async {
            client
                .vault_lock(key, TTL)
                .await
                .map_err(|error| Error::Lock { key, error })?;
            locked.push(key);
            let bytes = client
                .vault_get(key)
                .await
                .map_err(|error| Error::Get { key, error })?
                .ok_or(Error::Missing(key))?;
            let document: Document =
                serde_json::from_slice(&bytes).map_err(|_| Error::NotObject(key))?;
            Ok::<_, Error>((key, document))
        }
        .await;
        match result {
            Ok(document) => documents.push(document),
            Err(error) => {
                for key in locked {
                    let _ = client.vault_unlock(key).await;
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
                for &key in &keys {
                    // A refresh that fails is not this task's to
                    // report: the lock lapses by its TTL, and the
                    // release at the run's end reports what it finds.
                    let _ = client.vault_lock(key, TTL).await;
                }
            }
        }
    });
    Ok((
        documents,
        Held {
            client: Arc::clone(client),
            keys: locked,
            refresh,
        },
    ))
}

impl Held {
    /// Set every document as the run left it, and unlock every key.
    ///
    /// `documents` pairs each held key with the document read back
    /// from disk, or `None` for one no longer there — Hermes
    /// quarantines an entry whose refresh failed for good, and the
    /// caller keeping what it had is the honest answer; that key is
    /// unlocked without a set. Every key is attempted; the first
    /// failure is the one reported.
    pub async fn release(
        self,
        documents: Vec<(&'static str, Option<Vec<u8>>)>,
    ) -> Result<(), Error> {
        self.refresh.abort();
        let mut first: Option<Error> = None;
        for key in &self.keys {
            let body = documents
                .iter()
                .find(|(other, _)| other == key)
                .and_then(|(_, body)| body.as_deref());
            if let Some(body) = body {
                if let Err(error) = self.client.vault_set(key, body).await {
                    first.get_or_insert(Error::Set { key, error });
                }
            }
            if let Err(error) = self.client.vault_unlock(key).await {
                first.get_or_insert(Error::Unlock { key, error });
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

/// The cycle could not be completed.
#[derive(Debug)]
pub enum Error {
    /// The key could not be locked.
    Lock {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key could not be read.
    Get {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The vault holds nothing under the key.
    Missing(&'static str),
    /// The key holds something that is not a JSON object, which every
    /// state document here must be.
    NotObject(&'static str),
    /// The rotated document could not be set back.
    Set {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key could not be unlocked.
    Unlock {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Lock { key, error } => write!(f, "the vault key {key} could not be locked: {error}"),
            Error::Get { key, error } => write!(f, "the vault key {key} could not be read: {error}"),
            Error::Missing(key) => write!(f, "the vault holds no {key}"),
            Error::NotObject(key) => write!(f, "the vault's {key} is not a JSON object"),
            Error::Set { key, error } => write!(f, "the vault key {key} could not be set: {error}"),
            Error::Unlock { key, error } => write!(f, "the vault key {key} could not be unlocked: {error}"),
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
            Error::Missing(_) | Error::NotObject(_) => None,
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
