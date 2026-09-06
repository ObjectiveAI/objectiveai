//! What can go wrong between a program and the proxy's MCP server.

use std::error;
use std::fmt;

/// A session that could not be started, an exchange that could not
/// be carried, or a session that could not be ended cleanly.
///
/// What a tool SAID is never here: the proxy turns a tool's refusal
/// into a result the program reads, so these are only the link's
/// own failures.
#[derive(Debug)]
pub enum Error {
    /// Dialing or initializing the session failed.
    Connect(rmcp::service::ClientInitializeError),
    /// An exchange could not be carried: the session is gone, or
    /// the proxy answered something rmcp could not read.
    Service(rmcp::ServiceError),
    /// The session's task could not be joined on close.
    Close(tokio::task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connect(error) => {
                write!(f, "connecting to the proxy's MCP server failed: {error}")
            }
            Error::Service(error) => {
                write!(f, "an MCP exchange with the proxy failed: {error}")
            }
            Error::Close(error) => {
                write!(f, "closing the MCP session failed: {error}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Connect(error) => Some(error),
            Error::Service(error) => Some(error),
            Error::Close(error) => Some(error),
        }
    }
}
