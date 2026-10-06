//! Why a daemon connection's answer could not be decoded.

use std::error;
use std::fmt;

use crate::wire::frame;

/// A daemon connection's response that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this response's two.
    UnknownTag(u8),
    /// The server frame would not decode: shorter than its header, or
    /// of a type the wire does not have.
    Frame(frame::FrameError),
    /// An `Auth` frame, which does not pass.
    Auth,
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("daemon connection response is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown daemon connection response tag {tag}"),
            FrameError::Frame(error) => write!(f, "daemon connection server frame did not decode: {error}"),
            FrameError::Auth => f.write_str("an auth frame does not pass on a daemon connection"),
            FrameError::Error(error) => write!(f, "daemon connection error did not parse: {error}"),
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Frame(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) | FrameError::Auth => None,
        }
    }
}
