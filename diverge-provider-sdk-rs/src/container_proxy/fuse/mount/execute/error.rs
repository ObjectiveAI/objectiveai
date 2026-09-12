//! What can go wrong making a mount.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;
use crate::shared::containers::fuse::ResponseError;

/// The mount was not made, or its fate was not heard.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The proxy did not make the mount, and this is why.
    Refused(String),
    /// The answer would not decode.
    Frame(ResponseError),
    /// A close with nothing before it: could not serve, nothing said.
    Unserved,
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the proxy died.
    Closed,
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/fuse/mount: {error}"),
            ExecuteError::Encode(error) => write!(f, "mount request did not serialize: {error}"),
            ExecuteError::Refused(reason) => write!(f, "the proxy did not make the mount: {reason}"),
            ExecuteError::Frame(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve the mount"),
            ExecuteError::Socket(error) => write!(f, "/fuse/mount failed: {error}"),
            ExecuteError::Closed => f.write_str("/fuse/mount ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
            ExecuteError::Encode(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Socket(error) => Some(error),
            ExecuteError::Refused(_) | ExecuteError::Unserved | ExecuteError::Closed => None,
        }
    }
}
