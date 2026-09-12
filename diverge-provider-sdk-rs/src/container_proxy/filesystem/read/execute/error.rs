//! What can go wrong reading a file out.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;

/// The read could not be asked for.
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
            ExecuteError::Open(error) => write!(f, "/filesystem/read: {error}"),
            ExecuteError::Encode(error) => {
                write!(f, "read request did not serialize: {error}")
            }
            ExecuteError::Socket(error) => write!(f, "/filesystem/read failed: {error}"),
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

/// Why the read ended other than with the whole file. Every one is
/// terminal.
#[derive(Debug)]
pub enum ExecuteStreamError {
    /// The proxy did not read the file, or not all of it, and this is
    /// its reason.
    Refused(String),
    /// A close with nothing before it: could not serve, nothing said.
    Unserved,
    /// A message that would not decode.
    Frame(FrameError),
    /// The socket failed.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the read died.
    Closed,
}

impl fmt::Display for ExecuteStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteStreamError::Refused(reason) => {
                write!(f, "the proxy did not read the file: {reason}")
            }
            ExecuteStreamError::Unserved => f.write_str("the proxy could not serve the read"),
            ExecuteStreamError::Frame(error) => write!(f, "{error}"),
            ExecuteStreamError::Socket(error) => write!(f, "/filesystem/read failed: {error}"),
            ExecuteStreamError::Closed => f.write_str("/filesystem/read ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteStreamError::Frame(error) => Some(error),
            ExecuteStreamError::Socket(error) => Some(error),
            ExecuteStreamError::Refused(_)
            | ExecuteStreamError::Unserved
           
            | ExecuteStreamError::Closed => None,
        }
    }
}
