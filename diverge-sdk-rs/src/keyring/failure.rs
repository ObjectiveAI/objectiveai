//! What can go wrong at the keyring.

use std::error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// A keyring that could not be opened, a query that failed, or an
/// entry that is not as the call needs it.
#[derive(Debug)]
pub enum Error {
    /// The file could not be created owner-only.
    Create {
        /// The keyring file.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The file could not be opened as a SQLite database, or its
    /// table could not be made.
    Open {
        /// The keyring file.
        path: PathBuf,
        /// What SQLite said.
        source: sqlx::Error,
    },
    /// A query failed.
    Query(sqlx::Error),
    /// [`mint`](super::Keyring::mint) or [`set`](super::Keyring::set)
    /// was asked for a name already held.
    Exists(String),
    /// [`rotate`](super::Keyring::rotate) was asked for a name not
    /// held.
    Missing(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create { path, source } => write!(f, "creating the keyring {} failed: {source}", path.display()),
            Error::Open { path, source } => write!(f, "opening the keyring {} failed: {source}", path.display()),
            Error::Query(source) => write!(f, "the keyring query failed: {source}"),
            Error::Exists(name) => write!(f, "the keyring already holds `{name}`"),
            Error::Missing(name) => write!(f, "the keyring holds no `{name}`"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Create { source, .. } => Some(source),
            Error::Open { source, .. } | Error::Query(source) => Some(source),
            Error::Exists(_) | Error::Missing(_) => None,
        }
    }
}
