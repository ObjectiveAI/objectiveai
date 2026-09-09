//! The login: a mount, the vault's OAuth document, or the vault's
//! API key.
//!
//! The agent says nothing about how Codex logs in. At each run's
//! start [`Auth::resolve`] looks, in the ruled order:
//!
//! 1. A MOUNTED `auth.json` at [`AUTH_FILE`] — the caller's, of
//!    whatever kind, and the harness never reads it. A file this
//!    program itself wrote from the vault in an earlier run is NOT a
//!    mount ([`RENDERED`]), so the vault stays authoritative.
//! 2. The vault's [`OPENAI_CODEX_OAUTH`] — Codex's own `auth.json`,
//!    bytes verbatim, never rendered and no field ever guessed. Read
//!    without a lock and never written back: the document does not
//!    rotate in the container — it carries the access token the
//!    caller refreshes on their side — so there is no cycle to owe.
//! 3. The vault's `OPENAI_API_KEY` — static, into the process
//!    environment, where Codex's `env_key` looks.
//!
//! None of the three refuses the run, naming all three.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use diverge_container_proxy_sdk::Client;
use diverge_provider_sdk::shared::containers::vault::keys::OPENAI_CODEX_OAUTH;

/// Codex's home, fixed for the container's life: its default for the
/// root user, set explicitly on the process so the geometry never
/// moves.
pub const CODEX_HOME: &str = "/root/.codex";

/// Where Codex keeps its login, under the home.
pub const AUTH_FILE: &str = "/root/.codex/auth.json";

/// The static key an API-key login reads.
pub const API_KEY: &str = "OPENAI_API_KEY";

/// Whether this program has ever written [`AUTH_FILE`] from the
/// vault: a file it wrote is not a mount.
static RENDERED: AtomicBool = AtomicBool::new(false);

/// The login a run found.
pub enum Auth {
    /// A mounted `auth.json`: the caller's, left alone.
    Mounted,
    /// The vault's OAuth document, written as `auth.json`.
    Vault,
    /// The vault's API key, for the environment.
    ApiKey(String),
}

impl Auth {
    /// Look, in the ruled order.
    pub async fn resolve(client: &Arc<Client>) -> Result<Self, Error> {
        if !RENDERED.load(Ordering::SeqCst)
            && tokio::fs::try_exists(AUTH_FILE).await.unwrap_or(false)
        {
            return Ok(Auth::Mounted);
        }
        let document = client
            .vault_get(OPENAI_CODEX_OAUTH)
            .await
            .map_err(|error| Error::Get {
                key: OPENAI_CODEX_OAUTH,
                error,
            })?;
        if let Some(document) = document {
            if let Some(parent) = std::path::Path::new(AUTH_FILE).parent() {
                tokio::fs::create_dir_all(parent).await.map_err(Error::Write)?;
            }
            tokio::fs::write(AUTH_FILE, document.as_ref())
                .await
                .map_err(Error::Write)?;
            RENDERED.store(true, Ordering::SeqCst);
            return Ok(Auth::Vault);
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
            Auth::Mounted | Auth::Vault => None,
        }
    }
}

/// No login could be had.
#[derive(Debug)]
pub enum Error {
    /// None of the three places held a login.
    NoLogin,
    /// The key could not be read.
    Get {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key holds something that is not text.
    NotText(&'static str),
    /// `auth.json` could not be written.
    Write(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoLogin => write!(
                f,
                "no login: no {AUTH_FILE} is mounted, and the vault holds neither \
                 {OPENAI_CODEX_OAUTH} nor {API_KEY}"
            ),
            Error::Get { key, error } => {
                write!(f, "the vault key {key} could not be read: {error}")
            }
            Error::NotText(key) => write!(f, "the vault's {key} is not text"),
            Error::Write(error) => write!(f, "{AUTH_FILE} could not be written: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Get { error, .. } => Some(error),
            Error::Write(error) => Some(error),
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
