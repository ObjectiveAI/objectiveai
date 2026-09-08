//! A run that could not be driven at all.

use std::error;
use std::fmt;
use std::io;
use std::process::ExitStatus;

use crate::continuation;
use crate::filesystem::PrepareError;
use crate::vault;

/// Why the run never got going.
///
/// These are the failures with nothing to salvage: before the
/// gateway is up there is no session to speak of, and the stream's
/// one `Err` is the request's own failure, a status. Everything
/// after — a turn that fails, a stream that breaks, a harvest that
/// cannot happen — is said in the stream itself, as a fatal
/// `notification`.
#[derive(Debug)]
pub enum Error {
    /// The agent could not be planned, or the filesystem laid down.
    Prepare(PrepareError),
    /// A rotating document could not be had from the vault.
    Vault(vault::Error),
    /// The continuation could not be restored, or the session read.
    Continuation(continuation::Error),
    /// `hermes gateway` could not be spawned.
    Gateway(io::Error),
    /// The gateway exited before its API server ever answered.
    GatewayExited(ExitStatus),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Prepare(error) => write!(f, "{error}"),
            Error::Vault(error) => write!(f, "{error}"),
            Error::Continuation(error) => write!(f, "{error}"),
            Error::Gateway(error) => {
                write!(f, "the gateway process failed: {error}")
            }
            Error::GatewayExited(status) => {
                write!(f, "the gateway exited before it was ready: {status}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Prepare(error) => Some(error),
            Error::Vault(error) => Some(error),
            Error::Continuation(error) => Some(error),
            Error::Gateway(error) => Some(error),
            Error::GatewayExited(_) => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Vault(error) => error.message(),
            Error::Continuation(error) => error.message(),
            Error::Prepare(_) => serde_json::json!({
                "kind": "prepare",
                "error": self.to_string(),
            }),
            Error::Gateway(_) | Error::GatewayExited(_) => serde_json::json!({
                "kind": "gateway",
                "error": self.to_string(),
            }),
        }
    }
}

impl From<PrepareError> for Error {
    fn from(error: PrepareError) -> Self {
        Error::Prepare(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Gateway(error)
    }
}
