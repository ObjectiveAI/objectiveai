//! What can go wrong answering a fuse listing.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;
use crate::shared::containers::fuse::ResponseEncodeError;

/// The answer could not be delivered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened: `404` for a channel the proxy
    /// does not know, `409` for one already being answered.
    Open(OpenError),
    /// The listing would not encode: an entry's name too long for its
    /// prefix.
    Encode(ResponseEncodeError),
    /// The socket failed before the close: the answer died.
    Socket(tungstenite::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/fuse/list: {error}"),
            ExecuteError::Encode(error) => write!(f, "/fuse/list answer did not encode: {error}"),
            ExecuteError::Socket(error) => write!(f, "/fuse/list failed: {error}"),
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
