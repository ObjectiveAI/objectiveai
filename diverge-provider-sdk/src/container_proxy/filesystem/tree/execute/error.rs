//! What can go wrong watching the tree.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;

/// The watch could not be opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The socket failed before the request went out.
    Socket(tungstenite::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/filesystem/tree: {error}"),
            ExecuteError::Encode(error) => write!(f, "tree request did not serialize: {error}"),
            ExecuteError::Socket(error) => write!(f, "/filesystem/tree failed: {error}"),
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

/// Why the watch ended other than cleanly. Every one is terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The proxy could not watch: the watcher would not arm, the root
    /// would not be watched, the walk died — and this is its reason.
    Refused(String),
    /// A message that would not decode.
    Frame(FrameError),
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the proxy died.
    Closed,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Refused(reason) => {
                write!(f, "the proxy could not watch: {reason}")
            }
            ExecuteStreamError::Frame(error) => write!(f, "{error}"),
            ExecuteStreamError::Socket(error) => write!(f, "/filesystem/tree failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/filesystem/tree ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Socket(error) => Some(error),
            ExecuteStreamError::Refused(_) | ExecuteStreamError::Closed => None,
        }
    }
}
