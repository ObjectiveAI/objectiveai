//! The login: a mount, the vault's OAuth document, or the vault's
//! API key — and the cycle the rotating one owes.
//!
//! The agent says nothing about how Codex logs in. At each run's
//! start [`Auth::resolve`] looks, in the ruled order:
//!
//! 1. A MOUNTED `auth.json` at [`AUTH_FILE`] — the caller's, of
//!    whatever kind; Codex refreshes a ChatGPT login in place, on
//!    the mount, and the harness never reads it. A file this
//!    program itself rendered from the vault in an earlier run is
//!    NOT a mount ([`RENDERED`]), so the vault stays authoritative.
//! 2. The vault's [`OPENAI_CODEX_OAUTH`] — Codex's own `auth.json`,
//!    bytes verbatim, never rendered and no field ever guessed.
//!    ROTATING: Codex refreshes the tokens during use and rewrites
//!    the file, so the run owes hermes's cycle — lock for the run,
//!    refreshed before the TTL runs out; get; write the file; run;
//!    read the file back after every turn and set it when it
//!    changed; unlock at the end.
//! 3. The vault's `OPENAI_API_KEY` — static, into the process
//!    environment, where Codex's `env_key` looks.
//!
//! None of the three refuses the run, naming all three.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::shared::containers::vault::keys::OPENAI_CODEX_OAUTH;
use tokio::task::JoinHandle;

/// Codex's home, fixed for the container's life: its default for the
/// root user, set explicitly on the process so the geometry never
/// moves.
pub const CODEX_HOME: &str = "/root/.codex";

/// Where Codex keeps its login, under the home.
pub const AUTH_FILE: &str = "/root/.codex/auth.json";

/// The static key an API-key login reads.
pub const API_KEY: &str = "OPENAI_API_KEY";

/// How long the rotating key is locked for, in seconds.
pub const TTL: u32 = 300;

/// How often the held lock is refreshed — well inside the TTL.
pub const REFRESH: Duration = Duration::from_secs(100);

/// Whether this program has ever written [`AUTH_FILE`] from the
/// vault: a file it wrote is not a mount.
static RENDERED: AtomicBool = AtomicBool::new(false);

/// The login a run found.
pub enum Auth {
    /// A mounted `auth.json`: the caller's, left alone.
    Mounted,
    /// The vault's OAuth document, written as `auth.json` and held
    /// for the run.
    Vault(Held),
    /// The vault's API key, for the environment.
    ApiKey(String),
}

impl Auth {
    /// Look, in the ruled order.
    pub async fn resolve(client: &Arc<Client>) -> Result<Self, Error> {
        if !RENDERED.load(Ordering::SeqCst) && tokio::fs::try_exists(AUTH_FILE).await.unwrap_or(false)
        {
            return Ok(Auth::Mounted);
        }
        if let Some(held) = acquire(client).await? {
            return Ok(Auth::Vault(held));
        }
        let key = client
            .vault_get(API_KEY)
            .await
            .map_err(|error| Error::Get {
                key: API_KEY,
                error,
            })?;
        match key {
            Some(bytes) => String::from_utf8(bytes.to_vec())
                .map(Auth::ApiKey)
                .map_err(|_| Error::NotText(API_KEY)),
            None => Err(Error::NoLogin),
        }
    }

    /// The variable the process gets, when the login is a key.
    pub fn env(&self) -> Option<(&'static str, &str)> {
        match self {
            Auth::ApiKey(key) => Some((API_KEY, key.as_str())),
            Auth::Mounted | Auth::Vault(_) => None,
        }
    }
}

/// Lock the OAuth key and, if the vault holds it, write the document
/// as `auth.json`. `None` is a vault without it — the next place is
/// looked at — with the lock released again.
async fn acquire(client: &Arc<Client>) -> Result<Option<Held>, Error> {
    client
        .vault_lock(OPENAI_CODEX_OAUTH, TTL)
        .await
        .map_err(|error| Error::Lock {
            key: OPENAI_CODEX_OAUTH,
            error,
        })?;
    let result = async {
        let bytes = client
            .vault_get(OPENAI_CODEX_OAUTH)
            .await
            .map_err(|error| Error::Get {
                key: OPENAI_CODEX_OAUTH,
                error,
            })?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let document = bytes.to_vec();
        if let Some(parent) = std::path::Path::new(AUTH_FILE).parent() {
            tokio::fs::create_dir_all(parent).await.map_err(Error::Write)?;
        }
        tokio::fs::write(AUTH_FILE, &document).await.map_err(Error::Write)?;
        RENDERED.store(true, Ordering::SeqCst);
        Ok(Some(document))
    }
    .await;
    match result {
        Ok(Some(document)) => {
            let refresh = tokio::spawn({
                let client = Arc::clone(client);
                async move {
                    loop {
                        tokio::time::sleep(REFRESH).await;
                        // A refresh that fails is not this task's to
                        // report: the lock lapses by its TTL, and the
                        // release at the run's end reports what it
                        // finds.
                        let _ = client.vault_lock(OPENAI_CODEX_OAUTH, TTL).await;
                    }
                }
            });
            Ok(Some(Held {
                client: Arc::clone(client),
                last: document,
                refresh,
            }))
        }
        Ok(None) => {
            let _ = client.vault_unlock(OPENAI_CODEX_OAUTH).await;
            Ok(None)
        }
        Err(error) => {
            let _ = client.vault_unlock(OPENAI_CODEX_OAUTH).await;
            Err(error)
        }
    }
}

/// The OAuth key, held for the run: the last document set or read,
/// and the task keeping the lock.
#[must_use = "dropping the hold stops the refresh; the lock then lapses by TTL"]
pub struct Held {
    client: Arc<Client>,
    last: Vec<u8>,
    refresh: JoinHandle<()>,
}

impl Held {
    /// Read `auth.json` back as Codex left it and set it into the
    /// vault when it differs from the last document set or read.
    /// Answers whether it was set. A file no longer there is nothing
    /// new — the caller keeping its copy beats a lost login.
    pub async fn read_back(&mut self) -> Result<bool, Error> {
        let document = match tokio::fs::read(AUTH_FILE).await {
            Ok(document) => document,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(Error::Read(error)),
        };
        if document == self.last {
            return Ok(false);
        }
        self.client
            .vault_set(OPENAI_CODEX_OAUTH, &document)
            .await
            .map_err(|error| Error::Set {
                key: OPENAI_CODEX_OAUTH,
                error,
            })?;
        self.last = document;
        Ok(true)
    }

    /// Unlock the key.
    pub async fn release(self) -> Result<(), Error> {
        self.refresh.abort();
        self.client
            .vault_unlock(OPENAI_CODEX_OAUTH)
            .await
            .map_err(|error| Error::Unlock {
                key: OPENAI_CODEX_OAUTH,
                error,
            })
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // A hold dropped without release — the run abandoned — stops
        // refreshing; the lock lapses by its TTL.
        self.refresh.abort();
    }
}

/// No login could be had, or the cycle broke.
#[derive(Debug)]
pub enum Error {
    /// None of the three places held a login.
    NoLogin,
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
    /// The key holds something that is not text.
    NotText(&'static str),
    /// `auth.json` could not be written.
    Write(std::io::Error),
    /// `auth.json` could not be read back.
    Read(std::io::Error),
    /// The document could not be set back.
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
            Error::NoLogin => write!(
                f,
                "no login: no {AUTH_FILE} is mounted, and the vault holds neither \
                 {OPENAI_CODEX_OAUTH} nor {API_KEY}"
            ),
            Error::Lock { key, error } => {
                write!(f, "the vault key {key} could not be locked: {error}")
            }
            Error::Get { key, error } => {
                write!(f, "the vault key {key} could not be read: {error}")
            }
            Error::NotText(key) => write!(f, "the vault's {key} is not text"),
            Error::Write(error) => write!(f, "{AUTH_FILE} could not be written: {error}"),
            Error::Read(error) => write!(f, "{AUTH_FILE} could not be read back: {error}"),
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
            Error::Write(error) | Error::Read(error) => Some(error),
            Error::NoLogin | Error::NotText(_) => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "auth",
            "error": self.to_string(),
        })
    }
}
