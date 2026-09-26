//! What a begin could not do.

use std::fmt;

use crate::client::handle::SendError;
use crate::frame;

/// A begin that did not happen, or was not answered.
///
/// [`Refused`](Self::Refused) is the proxy's own answer and the rest
/// are this end's: the request that would not encode, the connection
/// gone, a frame this end cannot place, an answer it cannot read.
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
    Response(super::super::super::server::response::FrameError),
    /// The proxy would not begin: the connection had begun already,
    /// or the container's server refused the arguments. Its own words.
    Refused(crate::shared::error::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "begin request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "begin request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the proxy connection ended before the begin was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the proxy could not serve the begin"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a begin answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Refused(_) => f.write_str("the proxy refused the begin"),
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
