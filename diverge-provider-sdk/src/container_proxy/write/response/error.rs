//! Why the answer on `/write` could not be decoded.

use std::error;
use std::fmt;

/// An answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a kind.
    Empty,
    /// A kind this answer does not define.
    UnknownKind(u8),
    /// An error message that is not UTF-8.
    MessageUtf8,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("write answer is empty"),
            FrameError::UnknownKind(kind) => {
                write!(f, "unknown write answer kind {kind}")
            }
            FrameError::MessageUtf8 => {
                f.write_str("write error message is not utf-8")
            }
        }
    }
}

impl error::Error for FrameError {}
