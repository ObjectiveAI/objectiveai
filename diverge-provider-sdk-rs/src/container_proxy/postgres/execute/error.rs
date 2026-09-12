//! What can go wrong carrying a connection.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;

/// The conduit could not be opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// `404` for a connection the proxy did not announce, `409` for
    /// one already carried.
    Open(OpenError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/postgres: {error}"),
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

/// A chunk that could not be sent, or a close that could not be made.
#[derive(Debug)]
pub enum HandleError {
    /// The socket failed: the connection died.
    Socket(tungstenite::Error),
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::Socket(error) => write!(f, "/postgres failed: {error}"),
        }
    }
}

impl std::error::Error for HandleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleError::Socket(error) => Some(error),
        }
    }
}

/// Why the container's side ended other than cleanly. Every one is
/// terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the proxy died.
    Closed,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Socket(error) => write!(f, "/postgres failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/postgres ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Socket(error) => Some(error),
            ExecuteStreamError::Closed => None,
        }
    }
}
