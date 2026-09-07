//! Why a message on `/read` could not be decoded.

use std::error;
use std::fmt;

/// A read answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a kind.
    ///
    /// Distinct from an empty body, which is a kind byte followed by
    /// nothing and is an empty file's one message.
    Empty,
    /// A kind this answer does not define.
    UnknownKind(u8),
    /// An error message that is not UTF-8.
    MessageUtf8,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("read answer is empty"),
            FrameError::UnknownKind(kind) => {
                write!(f, "unknown read answer kind {kind}")
            }
            FrameError::MessageUtf8 => {
                f.write_str("read error message is not utf-8")
            }
        }
    }
}

impl error::Error for FrameError {}
