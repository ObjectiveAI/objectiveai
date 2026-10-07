//! Why a request over an agent could not be answered.

use std::fmt;

use crate::logs;
use crate::store;

/// The store, or the agent's log.
pub enum Failure {
    /// The records could not be read or written.
    Store(store::Error),
    /// The log could not be read, written or removed.
    Logs(logs::Error),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Store(error) => write!(f, "{error}"),
            Failure::Logs(error) => write!(f, "the log: {error}"),
        }
    }
}

impl From<store::Error> for Failure {
    fn from(error: store::Error) -> Self {
        Failure::Store(error)
    }
}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(store::Error::from(error))
    }
}

impl From<logs::Error> for Failure {
    fn from(error: logs::Error) -> Self {
        Failure::Logs(error)
    }
}
