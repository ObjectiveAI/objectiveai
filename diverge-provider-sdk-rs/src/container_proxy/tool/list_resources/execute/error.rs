//! What can go wrong asking the container's server.

use std::fmt;

use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;
use crate::shared::mcp::FrameError;

/// The exchange could not be made. The server's own refusal is not
/// here: that is the frame's `Error`, an answer.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The params would not serialize.
    Encode(serde_json::Error),
    /// The answer would not decode.
    Frame(FrameError),
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
            ExecuteError::Open(error) => write!(f, "/tool/list-resources: {error}"),
            ExecuteError::Encode(error) => write!(f, "/tool/list-resources params did not serialize: {error}"),
            ExecuteError::Frame(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve /tool/list-resources"),
            ExecuteError::Socket(error) => write!(f, "/tool/list-resources failed: {error}"),
            ExecuteError::Closed => f.write_str("/tool/list-resources ended without a close"),
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
            ExecuteError::Unserved | ExecuteError::Closed => None,
        }
    }
}
