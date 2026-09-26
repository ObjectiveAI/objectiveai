//! What a serve could not do.

use std::fmt;

use super::super::super::server::response;
use crate::wire::client::handle::SendError;
use crate::container_proxy::outside::fuse::mount::server::channel_request::FrameEncodeError;
use crate::wire::frame;
use crate::shared::error::Error;

/// A serve that did not open, or was not answered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request could not be sent.
    Send(SendError),
    /// The request did not serialize.
    Request(postcard::Error),
    /// The connection went away before the provider answered.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// The provider finished the scope with nothing before the finish:
    /// it could not serve the request, and said nothing else.
    Unanswered,
    /// A frame that cannot be the first on a main stream.
    Misrouted,
    /// The provider's answer did not decode.
    Response(response::FrameError),
    /// The provider refused to serve: a volume the caller cannot see,
    /// one held exclusively, one it could not open.
    Refused(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "volume serve request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "volume serve request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the connection ended before the volume serve was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the provider could not serve the volume"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a serve answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Refused(_) => f.write_str("the provider refused to serve the volume"),
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

/// An ask that has no answer.
#[derive(Debug)]
pub enum AskError {
    /// The ask did not encode: a path too long for its prefix.
    Encode(FrameEncodeError),
    /// The ask could not be sent: the scope, or the connection, is
    /// gone.
    Send(SendError),
    /// The provider finished the channel with nothing before the
    /// finish: it could not serve the ask.
    Unserved,
    /// The connection went away with the ask open.
    Closed,
}

impl fmt::Display for AskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AskError::Encode(error) => write!(f, "volume serve ask did not encode: {error}"),
            AskError::Send(error) => write!(f, "volume serve ask could not be sent: {error}"),
            AskError::Unserved => f.write_str("the provider could not serve the ask"),
            AskError::Closed => f.write_str("the connection ended with the ask open"),
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
