//! A delivered database that did not check out.

use std::error;
use std::fmt;

/// A delivered database that did not check out.
#[derive(Debug)]
pub enum CheckError {
    /// The database could not be opened or asked.
    Sqlite(sqlx::Error),
    /// The database opened but did not check out: `quick_check`'s
    /// findings, verbatim.
    Corrupt(Vec<String>),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Sqlite(error) => {
                write!(f, "the delivered state.db could not be opened: {error}")
            }
            CheckError::Corrupt(findings) => write!(
                f,
                "the delivered state.db is corrupt: {}",
                findings.join("; ")
            ),
        }
    }
}

impl error::Error for CheckError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            CheckError::Sqlite(error) => Some(error),
            CheckError::Corrupt(_) => None,
        }
    }
}

impl From<sqlx::Error> for CheckError {
    fn from(error: sqlx::Error) -> Self {
        CheckError::Sqlite(error)
    }
}
