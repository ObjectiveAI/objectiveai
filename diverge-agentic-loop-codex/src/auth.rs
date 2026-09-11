//! The login: a mounted `auth.json`, or the vault's API key.
//!
//! The agent says nothing about how Codex logs in. At each run's
//! start [`Auth::resolve`] looks, in the ruled order:
//!
//! 1. A MOUNTED `auth.json` at [`AUTH_FILE`] — the caller's, of
//!    whatever kind, served through a FUSE mount on the container
//!    request, and the harness never reads it. Codex refreshes a
//!    ChatGPT login in place, on the mount, so the caller's copy is
//!    the current one without any cycle of the harness's.
//! 2. The vault's `OPENAI_API_KEY` — static, into the process
//!    environment, where Codex's `env_key` looks.
//!
//! Neither refuses the run, naming both. The vault's OAuth document
//! is NOT a source: a login that refreshes is the caller's to serve
//! live, which is what the mount is for, and nothing here writes an
//! `auth.json` of its own.

use std::sync::Arc;

use diverge_container_proxy_sdk::Client;

/// Codex's home, fixed for the container's life: its default for the
/// root user, set explicitly on the process so the geometry never
/// moves.
pub const CODEX_HOME: &str = "/root/.codex";

/// Where Codex keeps its login, under the home.
pub const AUTH_FILE: &str = "/root/.codex/auth.json";

/// The static key an API-key login reads.
pub const API_KEY: &str = "OPENAI_API_KEY";

/// The login a run found.
pub enum Auth {
    /// A mounted `auth.json`: the caller's, left alone.
    Mounted,
    /// The vault's API key, for the environment.
    ApiKey(String),
}

impl Auth {
    /// Look, in the ruled order.
    pub async fn resolve(client: &Arc<Client>) -> Result<Self, Error> {
        if tokio::fs::try_exists(AUTH_FILE).await.unwrap_or(false) {
            return Ok(Auth::Mounted);
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
            Auth::Mounted => None,
        }
    }
}

/// No login could be had.
#[derive(Debug)]
pub enum Error {
    /// Neither place held a login.
    NoLogin,
    /// The key could not be read.
    Get {
        key: &'static str,
        error: diverge_container_proxy_sdk::Error,
    },
    /// The key holds something that is not text.
    NotText(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoLogin => write!(
                f,
                "no login: no {AUTH_FILE} is mounted, and the vault holds no {API_KEY}"
            ),
            Error::Get { key, error } => {
                write!(f, "the vault key {key} could not be read: {error}")
            }
            Error::NotText(key) => write!(f, "the vault's {key} is not text"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Get { error, .. } => Some(error),
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
