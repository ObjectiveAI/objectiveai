//! What can go wrong answering a command.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;

/// The path could not be opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// `404` for a channel the proxy does not know, `409` for one
    /// already being answered.
    Open(OpenError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/command: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
        }
    }
}

/// A frame that could not be sent, or a close that could not be made.
#[derive(Debug)]
pub enum HandleError {
    /// The frame would not serialize.
    Encode(serde_json::Error),
    /// The socket failed: the answer died.
    Socket(tungstenite::Error),
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::Encode(error) => {
                write!(f, "frame did not serialize: {error}")
            }
            HandleError::Socket(error) => write!(f, "/command failed: {error}"),
        }
    }
}

impl std::error::Error for HandleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleError::Encode(error) => Some(error),
            HandleError::Socket(error) => Some(error),
        }
    }
}
