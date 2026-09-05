//! Why a message on `/command/{channel}` could not be decoded.

use std::error;
use std::fmt;

/// A command response that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from an empty item, which is a tag byte followed by
    /// nothing and is a command that produced something with no bytes
    /// in it.
    Empty,
    /// A tag that is neither of this response's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("command response is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown command response tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "command error did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
