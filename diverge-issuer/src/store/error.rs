//! Why the database refused.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What the [`Store`](super::Store) fails with.
#[derive(Debug)]
pub enum Error {
    /// The file could not be created.
    Create {
        /// The file.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The file could not be opened, or its tables made.
    Open {
        /// The file.
        path: PathBuf,
        /// Why.
        source: sqlx::Error,
    },
    /// A query failed.
    Query(sqlx::Error),
    /// The name is held already: an identity's subject.
    Exists(String),
    /// What was asked for is refused as it was put: why.
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Create { path, source } => write!(f, "`{}` could not be created: {source}", path.display()),
            Error::Open { path, source } => write!(f, "`{}` could not be opened: {source}", path.display()),
            Error::Query(error) => write!(f, "the issuer's database refused: {error}"),
            Error::Exists(name) => write!(f, "`{name}` exists already"),
            Error::Invalid(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Create { source, .. } => Some(source),
            Error::Open { source, .. } => Some(source),
            Error::Query(error) => Some(error),
            Error::Exists(_) | Error::Invalid(_) => None,
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Error::Query(error)
    }
}
