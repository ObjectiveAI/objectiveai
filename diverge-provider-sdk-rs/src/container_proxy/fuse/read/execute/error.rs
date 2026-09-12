//! What can go wrong answering a fuse read.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;

/// The answer could not be delivered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened: `404` for a channel the proxy
    /// does not know, `409` for one already being answered.
    Open(OpenError),
    /// The socket failed before the close: the answer died.
    Socket(tungstenite::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/fuse/read: {error}"),
            ExecuteError::Socket(error) => write!(f, "/fuse/read failed: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
            ExecuteError::Socket(error) => Some(error),
        }
    }
}
