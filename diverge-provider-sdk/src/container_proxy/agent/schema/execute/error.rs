//! What can go wrong asking for the schema.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;
use crate::shared::error::Error;

/// The schema could not be had.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The container's own `Error`: no schema to give, in its words.
    Refused(Error),
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
            ExecuteError::Open(error) => write!(f, "/agent/schema: {error}"),
            ExecuteError::Refused(error) => {
                write!(f, "the container gave no schema: {}", error.0)
            }
            ExecuteError::Frame(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve the schema"),
            ExecuteError::Socket(error) => write!(f, "/agent/schema failed: {error}"),
            ExecuteError::Closed => f.write_str("/agent/schema ended without a close"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Socket(error) => Some(error),
            ExecuteError::Refused(_)
            | ExecuteError::Unserved
           
            | ExecuteError::Closed => None,
        }
    }
}
