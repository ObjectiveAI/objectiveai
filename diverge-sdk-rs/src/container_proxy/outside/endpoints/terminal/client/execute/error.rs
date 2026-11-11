//! Why a terminal did not open.

use std::fmt;

use crate::wire::client::handle::SendError;

/// The request did not go out.
#[derive(Debug)]
pub enum ExecuteError {
    /// The wire did not take it.
    Send(SendError),
    /// It did not serialize.
    Request(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "terminal request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "terminal request did not serialize: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
        }
    }
}
