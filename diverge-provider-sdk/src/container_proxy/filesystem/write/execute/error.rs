//! What can go wrong writing a file in.

use std::fmt;

use tokio_tungstenite::tungstenite;

use super::super::response::FrameError;
use crate::server::container_client::OpenError;

/// The file did not land, and why.
///
/// Generic over the content stream's own error, which comes back as
/// [`Content`](Self::Content) untouched: what went wrong reading the
/// source is the source's to say.
#[derive(Debug)]
pub enum ExecuteError<E> {
    /// The path could not be opened.
    Open(OpenError),
    /// The request would not serialize.
    Encode(serde_json::Error),
    /// The content stream failed; the write was abandoned and the
    /// destination is untouched.
    Content(E),
    /// The proxy did not write the file, and this is its reason.
    Refused(String),
    /// The answer would not decode.
    Answer(FrameError),
    /// A close with no answer: could not serve, nothing said.
    Unserved,
    /// The socket failed: the write died, its outcome unknown.
    Socket(tungstenite::Error),
    /// The socket ended without a Close: the write died, its outcome
    /// unknown.
    Closed,
}

impl<E: fmt::Display> fmt::Display for ExecuteError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Open(error) => write!(f, "/filesystem/write: {error}"),
            ExecuteError::Encode(error) => {
                write!(f, "write request did not serialize: {error}")
            }
            ExecuteError::Content(error) => write!(f, "content failed: {error}"),
            ExecuteError::Refused(reason) => {
                write!(f, "the proxy did not write the file: {reason}")
            }
            ExecuteError::Answer(error) => write!(f, "{error}"),
            ExecuteError::Unserved => f.write_str("the proxy could not serve the write"),
            ExecuteError::Socket(error) => write!(f, "/filesystem/write failed: {error}"),
            ExecuteError::Closed => f.write_str("/filesystem/write ended without a close"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ExecuteError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Open(error) => Some(error),
            ExecuteError::Encode(error) => Some(error),
            ExecuteError::Content(error) => Some(error),
            ExecuteError::Answer(error) => Some(error),
            ExecuteError::Socket(error) => Some(error),
            ExecuteError::Refused(_)
            | ExecuteError::Unserved
           
            | ExecuteError::Closed => None,
        }
    }
}
