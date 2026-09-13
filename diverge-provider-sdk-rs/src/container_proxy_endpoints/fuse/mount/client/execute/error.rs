//! What a mount could not do.

use std::fmt;

use crate::client::handle::SendError;
use crate::frame;

/// A mount that did not happen, or was not answered.
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
    Response(crate::shared::containers::fuse::ResponseError),
    /// The proxy did not make the mount, and this is why: the path empty or the root, a path already mounted, a mount point it could not make, no FUSE on the host, a session that would not start.
    Refused(String),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "mount request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "mount request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the proxy connection ended before the mount was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the proxy could not serve the mount"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a mount answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Refused(_) => f.write_str("the proxy refused the mount"),
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
