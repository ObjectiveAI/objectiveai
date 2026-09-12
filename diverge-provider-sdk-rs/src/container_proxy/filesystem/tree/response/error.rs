//! Why a message on `/filesystem/tree` could not be decoded.

use std::error;
use std::fmt;

/// A filetree message that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a kind.
    Empty,
    /// A kind this answer does not define.
    UnknownKind(u8),
    /// The postcard payload would not decode as a filetree frame.
    Filetree(postcard::Error),
    /// An error message that is not UTF-8.
    MessageUtf8,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("filetree message is empty"),
            FrameError::UnknownKind(kind) => {
                write!(f, "unknown filetree message kind {kind}")
            }
            FrameError::Filetree(error) => {
                write!(f, "filetree frame did not decode: {error}")
            }
            FrameError::MessageUtf8 => {
                f.write_str("filetree error message is not utf-8")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Filetree(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownKind(_)
            | FrameError::MessageUtf8 => None,
        }
    }
}
