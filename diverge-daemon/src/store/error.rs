//! Why a record could not be read or written.

use std::fmt;
use std::net::AddrParseError;

/// What every store call fails with: the database, or a row holding
/// something that is not what it should be.
#[derive(Debug)]
pub enum Error {
    /// The database refused, or could not be reached.
    Database(sqlx::Error),
    /// A JSON column — a creator, a role's grants — did not parse as
    /// the type it holds.
    Json(serde_json::Error),
    /// An address column did not parse as an address.
    Address {
        /// What the column held.
        value: String,
        /// Why it is not an address.
        source: AddrParseError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Database(error) => write!(f, "the database: {error}"),
            Error::Json(error) => write!(f, "a record's JSON did not parse: {error}"),
            Error::Address { value, source } => write!(f, "the address `{value}` did not parse: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(error) => Some(error),
            Error::Json(error) => Some(error),
            Error::Address { source, .. } => Some(source),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Error::Database(error)
    }
}
