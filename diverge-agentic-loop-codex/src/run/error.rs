//! A run that could not be driven at all.

use std::error;
use std::fmt;
use std::io;
use std::process::ExitStatus;

use super::LineError;
use crate::auth;
use crate::continuation;

/// Why the run never got going.
///
/// These are the failures with nothing to salvage: before the first
/// process has written a single event no turn has run, and the
/// stream's one `Err` is the request's own failure, a status.
/// Everything after — a turn that fails, a process that dies, a
/// document that will not set, a harvest that cannot happen — is
/// said in the stream itself, as a fatal `notification`.
#[derive(Debug)]
pub enum Error {
    /// No login could be had.
    Auth(auth::Error),
    /// `config.toml` could not be rendered or written.
    Config(io::Error),
    /// The continuation could not be loaded or restored.
    Continuation(continuation::Error),
    /// `codex exec` could not be spawned, or spoken to.
    Spawn(io::Error),
    /// The first process exited before writing a single event.
    Exited(ExitStatus),
    /// The first process wrote something before any event that is
    /// not an event.
    Line(LineError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Auth(error) => write!(f, "{error}"),
            Error::Config(error) => write!(f, "config.toml could not be written: {error}"),
            Error::Continuation(error) => write!(f, "{error}"),
            Error::Spawn(error) => write!(f, "codex could not be spawned: {error}"),
            Error::Exited(status) => {
                write!(f, "codex exited before writing an event: {status}")
            }
            Error::Line(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Auth(error) => Some(error),
            Error::Config(error) | Error::Spawn(error) => Some(error),
            Error::Continuation(error) => Some(error),
            Error::Line(error) => Some(error),
            Error::Exited(_) => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Auth(error) => error.message(),
            Error::Continuation(error) => error.message(),
            Error::Config(_) => serde_json::json!({
                "kind": "config",
                "error": self.to_string(),
            }),
            Error::Spawn(_) | Error::Exited(_) | Error::Line(_) => serde_json::json!({
                "kind": "codex",
                "error": self.to_string(),
            }),
        }
    }
}

impl From<LineError> for Error {
    fn from(error: LineError) -> Self {
        match error {
            LineError::Io(error) => Error::Spawn(error),
            error => Error::Line(error),
        }
    }
}
