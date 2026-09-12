//! What can go wrong subscribing.

use std::fmt;

use rmcp::ErrorData;
use tokio_tungstenite::tungstenite;

use crate::server::container_client::OpenError;
use crate::shared::mcp::FrameError;

/// The subscription could not be opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/tool/notifications: {error}"),
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

/// Why the subscription ended other than by the proxy closing it.
/// Every one is terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The proxy's `Error` frame: the container's server could not be
    /// reached, in the vocabulary the exchange uses. The last thing on
    /// the stream.
    Refused(ErrorData),
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
            ExecuteStreamError::Refused(error) => write!(f, "the subscription was refused: {}", error.message),
            ExecuteStreamError::Frame(error) => write!(f, "{error}"),
            ExecuteStreamError::Socket(error) => write!(f, "/tool/notifications failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/tool/notifications ended without a close"),
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
