//! What a read could not do before its scope was open.

use std::fmt;

use crate::wire::client::handle::SendError;

/// A read that could not be opened. Everything after the opening
/// is the stream's to report.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request could not be sent.
    Send(SendError),
    /// The request did not serialize.
    Request(serde_json::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "read request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "read request did not serialize: {error}"),
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
