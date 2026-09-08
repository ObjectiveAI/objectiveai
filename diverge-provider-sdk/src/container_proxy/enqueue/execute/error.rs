//! What can go wrong enqueuing a message.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;

/// The fate could not be had.
///
/// Not among these: the agent's server's own `Error` frame, which
/// comes back as the answer it is.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The answer would not decode.
    Frame(FrameError),
    /// A close with nothing before it: could not serve, nothing said.
    Unserved,
    /// A text message: the far side speaking something else.
    Text,
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the proxy died.
    Closed,
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/agent/enqueue: {error}"),
            ExecuteError::Encode(error) => {
                write!(f, "enqueue request did not serialize: {error}")
            }
            ExecuteError::Frame(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve the enqueue"),
            ExecuteError::Text => f.write_str("/agent/enqueue carried a text message"),
            ExecuteError::Socket(error) => write!(f, "/agent/enqueue failed: {error}"),
            ExecuteError::Closed => f.write_str("/agent/enqueue ended without a close"),
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
            ExecuteError::Unserved | ExecuteError::Text | ExecuteError::Closed => None,
        }
    }
}
