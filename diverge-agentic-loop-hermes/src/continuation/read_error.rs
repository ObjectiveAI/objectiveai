//! A harvest that could not happen.

use std::error;
use std::fmt;
use std::io;

/// A harvest that could not happen.
#[derive(Debug)]
pub enum ReadError {
    /// A file could not be read, or a sidecar inspected or removed.
    Io(io::Error),
    /// The database could not be opened, vacuumed, checkpointed or
    /// closed.
    Sqlite(sqlx::Error),
    /// The checkpoint was blocked: something still holds the
    /// database, and the gateway was supposed to be gone.
    Busy,
    /// The write-ahead log still carries this many bytes after the
    /// close: the fold did not happen, and the main file alone would
    /// lose committed transactions.
    WalRemains(u64),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Io(error) => {
                write!(f, "the continuation could not be read: {error}")
            }
            ReadError::Sqlite(error) => {
                write!(f, "state.db could not be folded: {error}")
            }
            ReadError::Busy => f.write_str(
                "the state.db checkpoint was blocked: something still holds the database",
            ),
            ReadError::WalRemains(len) => write!(
                f,
                "the state.db write-ahead log still holds {len} bytes after the fold"
            ),
        }
    }
}

impl error::Error for ReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ReadError::Io(error) => Some(error),
            ReadError::Sqlite(error) => Some(error),
            ReadError::Busy | ReadError::WalRemains(_) => None,
        }
    }
}

impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        ReadError::Io(error)
    }
}

impl From<sqlx::Error> for ReadError {
    fn from(error: sqlx::Error) -> Self {
        ReadError::Sqlite(error)
    }
}
