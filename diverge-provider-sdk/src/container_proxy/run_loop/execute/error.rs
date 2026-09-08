//! What can go wrong running the loop.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;
use crate::shared::error::Error;

/// The loop could not be asked for.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened: `409` for a loop already in
    /// progress.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The socket failed before the request went out.
    Socket(tungstenite::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/run-loop: {error}"),
            ExecuteError::Encode(error) => {
                write!(f, "run-loop request did not serialize: {error}")
            }
            ExecuteError::Socket(error) => write!(f, "/run-loop failed: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
            ExecuteError::Encode(error) => Some(error),
            ExecuteError::Socket(error) => Some(error),
        }
    }
}

/// Why the loop ended other than by finishing. Every one is terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The container's own `Error` frame: there was no loop to report
    /// on, or it died with nothing more to say. The JSON value it
    /// chose, as it chose it.
    Refused(Error),
    /// A message that would not decode.
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
            ExecuteStreamError::Refused(error) => {
                write!(f, "the container refused the loop: {}", error.0)
            }
            ExecuteStreamError::Frame(error) => write!(f, "{error}"),
            ExecuteStreamError::Text => f.write_str("/run-loop carried a text message"),
            ExecuteStreamError::Socket(error) => write!(f, "/run-loop failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/run-loop ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Socket(error) => Some(error),
            ExecuteStreamError::Refused(_)
            | ExecuteStreamError::Text
            | ExecuteStreamError::Closed => None,
        }
    }
}
