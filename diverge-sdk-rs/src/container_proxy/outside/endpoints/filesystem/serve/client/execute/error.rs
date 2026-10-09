//! What a serve could not do.

use std::fmt;

use super::super::super::server::response;
use crate::wire::client::handle::SendError;
use crate::container_proxy::outside::endpoints::fuse::mount::server::channel_request::FrameEncodeError;
use crate::wire::frame;
use crate::shared::error::Error;

/// A serve that did not open, or was not answered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request could not be sent.
    Send(SendError),
    /// The request did not serialize.
    Request(serde_json::Error),
    /// The connection went away before the proxy answered.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// The proxy finished the scope with nothing before the finish:
    /// it could not serve the request, and said nothing else.
    Unanswered,
    /// A frame that cannot be the first on a main stream.
    Misrouted,
    /// The proxy's answer did not decode.
    Response(response::FrameError),
    /// The proxy refused to serve: a path that is not a directory of
    /// the container, or one the proxy leaves out; its own words.
    Refused(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "serve request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "serve request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the proxy connection ended before the serve was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the proxy could not serve the subtree"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a serve answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Refused(_) => f.write_str("the proxy refused to serve the subtree"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::Misrouted | ExecuteError::Refused(_) => None,
        }
    }
}

/// An ask that was not answered.
#[derive(Debug)]
pub enum AskError {
    /// The ask did not encode: a path too long for its prefix.
    Encode(FrameEncodeError),
    /// The ask could not be sent: the scope, or the connection, is
    /// gone.
    Send(SendError),
    /// The proxy finished the channel with nothing before the finish:
    /// it could not serve the ask.
    Unserved,
    /// The connection went away with the ask open.
    Closed,
}

impl fmt::Display for AskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AskError::Encode(error) => write!(f, "serve ask did not encode: {error}"),
            AskError::Send(error) => write!(f, "serve ask could not be sent: {error}"),
            AskError::Unserved => f.write_str("the proxy could not serve the ask"),
            AskError::Closed => f.write_str("the proxy connection ended with the ask open"),
        }
    }
}

impl std::error::Error for AskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AskError::Encode(error) => Some(error),
            AskError::Send(error) => Some(error),
            AskError::Unserved | AskError::Closed => None,
        }
    }
}
