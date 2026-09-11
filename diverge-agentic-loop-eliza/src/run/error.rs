//! A run that could not be driven at all.

use std::error;
use std::fmt;
use std::io;
use std::process::ExitStatus;

use super::LineError;
use crate::lineage;
use crate::plugins;
use crate::vault;

/// Why the run never got going.
///
/// These are the failures with nothing to salvage: before the entry
/// says `ready` no turn has run, and the stream's one `Err` is the
/// request's own failure, a status. Everything after — a turn that
/// fails, an entry that dies, a secret that will not set — is said in
/// the stream itself, as a fatal `notification`.
#[derive(Debug)]
pub enum Error {
    /// The lineage row could not be read or written.
    Lineage(lineage::Error),
    /// A secret could not be had from the vault.
    Vault(vault::Error),
    /// A plugin could not be installed.
    Install(plugins::Error),
    /// The entry could not be spawned, or spoken to.
    Entry(io::Error),
    /// The entry failed before it was ready, in its own words.
    Fatal(String),
    /// The entry exited before it was ready, saying nothing.
    Exited(ExitStatus),
    /// The entry wrote something before ready that is not a response.
    Line(LineError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lineage(error) => write!(f, "{error}"),
            Error::Vault(error) => write!(f, "{error}"),
            Error::Install(error) => write!(f, "{error}"),
            Error::Entry(error) => write!(f, "the entry process failed: {error}"),
            Error::Fatal(error) => write!(f, "the entry failed before it was ready: {error}"),
            Error::Exited(status) => {
                write!(f, "the entry exited before it was ready: {status}")
            }
            Error::Line(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Lineage(error) => Some(error),
            Error::Vault(error) => Some(error),
            Error::Install(error) => Some(error),
            Error::Entry(error) => Some(error),
            Error::Line(error) => Some(error),
            Error::Fatal(_) | Error::Exited(_) => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Lineage(error) => error.message(),
            Error::Vault(error) => error.message(),
            Error::Install(error) => error.message(),
            Error::Entry(_) | Error::Fatal(_) | Error::Exited(_) | Error::Line(_) => {
                serde_json::json!({
                    "kind": "entry",
                    "error": self.to_string(),
                })
            }
        }
    }
}

impl From<lineage::Error> for Error {
    fn from(error: lineage::Error) -> Self {
        Error::Lineage(error)
    }
}

impl From<vault::Error> for Error {
    fn from(error: vault::Error) -> Self {
        Error::Vault(error)
    }
}

impl From<plugins::Error> for Error {
    fn from(error: plugins::Error) -> Self {
        Error::Install(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Entry(error)
    }
}

impl From<LineError> for Error {
    fn from(error: LineError) -> Self {
        match error {
            LineError::Io(error) => Error::Entry(error),
            error => Error::Line(error),
        }
    }
}
