//! What can go wrong registering the agent.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;
use crate::shared::error::Error;

/// The agent could not be registered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The path could not be opened.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The container's own `Error`: it would not take the agent, in
    /// its words.
    Refused(Error),
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
            ExecuteError::Open(error) => write!(f, "/agent/register: {error}"),
            ExecuteError::Encode(error) => {
                write!(f, "register request did not serialize: {error}")
            }
            ExecuteError::Refused(error) => {
                write!(f, "the container refused the agent: {}", error.0)
            }
            ExecuteError::Frame(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve the registration"),
            ExecuteError::Text => f.write_str("/agent/register carried a text message"),
            ExecuteError::Socket(error) => write!(f, "/agent/register failed: {error}"),
            ExecuteError::Closed => f.write_str("/agent/register ended without a close"),
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
            ExecuteError::Refused(_)
            | ExecuteError::Unserved
            | ExecuteError::Text
            | ExecuteError::Closed => None,
        }
    }
}
