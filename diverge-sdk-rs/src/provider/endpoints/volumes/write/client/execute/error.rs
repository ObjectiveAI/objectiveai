//! What a write could not do.

use std::fmt;

use crate::wire::client::handle::SendError;
use crate::wire::frame;

/// A write that did not happen, or was not answered.
///
/// [`Refused`](Self::Refused) is the provider's own answer and the
/// rest are this end's: the request that would not encode, the
/// connection gone, a frame this end cannot place, an answer it
/// cannot read.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request could not be sent.
    Send(SendError),
    /// The request did not serialize.
    Request(postcard::Error),
    /// A content frame did not serialize.
    Content(serde_json::Error),
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
    Response(super::super::super::server::response::FrameError),
    /// The file did not land, and nothing partial did: the provider's
    /// own words.
    Refused(crate::shared::error::Error),
    /// The content ended in an error before it was all sent: the
    /// write is abandoned, and the provider discards what it wrote.
    Source(crate::shared::error::Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => {
                write!(f, "volume write request could not be sent: {error}")
            }
            ExecuteError::Request(error) => {
                write!(f, "volume write request did not serialize: {error}")
            }
            ExecuteError::Content(error) => {
                write!(f, "volume write content did not serialize: {error}")
            }
            ExecuteError::Closed => {
                f.write_str("the connection ended before the volume write was answered")
            }
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => {
                f.write_str("the provider could not serve the volume write")
            }
            ExecuteError::Misrouted => {
                f.write_str("a frame that cannot open a write answered it")
            }
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Refused(_) => f.write_str("the provider refused the volume write"),
            ExecuteError::Source(_) => {
                f.write_str("the content ended in an error before the write was whole")
            }
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Content(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Misrouted
            | ExecuteError::Refused(_)
            | ExecuteError::Source(_) => None,
        }
    }
}
