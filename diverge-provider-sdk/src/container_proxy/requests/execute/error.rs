//! What can go wrong taking `/requests`.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::request::FrameError;
use crate::server::container_client::OpenError;

/// `/requests` could not be taken.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened — `409` is another server holding
    /// it.
    Open(OpenError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/requests: {error}"),
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

/// Why the asks ended other than cleanly. Every one is terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// A frame that would not decode: the container speaking something
    /// this version does not know.
    Frame(FrameError),
    /// A text message: the far side speaking something else.
    Text,
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the proxy died.
    Closed,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Frame(error) => write!(f, "{error}"),
            ExecuteStreamError::Text => f.write_str("/requests carried a text message"),
            ExecuteStreamError::Socket(error) => write!(f, "/requests failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/requests ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Socket(error) => Some(error),
            ExecuteStreamError::Text | ExecuteStreamError::Closed => None,
        }
    }
}
