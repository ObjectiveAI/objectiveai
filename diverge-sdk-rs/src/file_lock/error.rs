//! What can go wrong taking a lock.

use std::error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// A lock that could not be taken. Each variant names the file, since
/// a caller holding several is told which.
#[derive(Debug)]
pub enum Error {
    /// The file could not be created or opened for locking.
    Open {
        /// The lock file.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The lock call itself failed — not "somebody else holds it",
    /// which [`try_exclusive`](super::try_exclusive) answers with
    /// `None`, but a failure of the call.
    Lock {
        /// The lock file.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The blocking task the lock was taken on did not finish: it
    /// panicked, or the runtime is shutting down.
    Blocking {
        /// The lock file.
        path: PathBuf,
        /// Why the task did not finish.
        source: tokio::task::JoinError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Open { path, source } => {
                write!(f, "opening the lock file {} failed: {source}", path.display())
            }
            Error::Lock { path, source } => {
                write!(f, "locking {} failed: {source}", path.display())
            }
            Error::Blocking { path, source } => {
                write!(f, "the task locking {} did not finish: {source}", path.display())
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Open { source, .. } | Error::Lock { source, .. } => Some(source),
            Error::Blocking { source, .. } => Some(source),
        }
    }
}
