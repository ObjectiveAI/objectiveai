//! A transcript that could not be read back.

use std::error;
use std::fmt;

/// Why a session's transcript could not be read.
#[derive(Debug)]
pub enum HistoryError {
    /// The database could not be opened, queried, or closed.
    Sqlite(sqlx::Error),
    /// A content column carries the JSON sentinel but not JSON —
    /// a corrupt row, which is a corrupt continuation. Hermes logs
    /// and keeps the raw string; this refuses.
    Content(serde_json::Error),
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistoryError::Sqlite(error) => {
                write!(f, "the session transcript could not be read: {error}")
            }
            HistoryError::Content(error) => {
                write!(f, "a stored message's content is not JSON: {error}")
            }
        }
    }
}

impl error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            HistoryError::Sqlite(error) => Some(error),
            HistoryError::Content(error) => Some(error),
        }
    }
}

impl From<sqlx::Error> for HistoryError {
    fn from(error: sqlx::Error) -> Self {
        HistoryError::Sqlite(error)
    }
}
