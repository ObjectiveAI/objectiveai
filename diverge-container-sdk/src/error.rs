//! What can go wrong between a program and the proxy.

use std::error;
use std::fmt;

/// A failure of the client's own link to the proxy.
///
/// What a tool SAID is never here: the proxy turns a tool's refusal
/// into a result the program reads. Variants are named for the
/// feature they belong to, so a program handling one knows which
/// connection it was.
#[derive(Debug)]
pub enum Error {
    /// Dialing or initializing the MCP session failed.
    McpConnect(rmcp::service::ClientInitializeError),
    /// An MCP exchange could not be carried: the session is gone, or
    /// the proxy answered something rmcp could not read.
    McpService(rmcp::ServiceError),
    /// The MCP session's task could not be joined on close.
    McpClose(tokio::task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::McpConnect(error) => {
                write!(f, "connecting to the proxy's MCP server failed: {error}")
            }
            Error::McpService(error) => {
                write!(f, "an MCP exchange with the proxy failed: {error}")
            }
            Error::McpClose(error) => {
                write!(f, "closing the MCP session failed: {error}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::McpConnect(error) => Some(error),
            Error::McpService(error) => Some(error),
            Error::McpClose(error) => Some(error),
        }
    }
}
