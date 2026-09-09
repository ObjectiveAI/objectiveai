//! Why a fuse ask or answer could not be read or written.

use std::error;
use std::fmt;

/// A fuse ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// Fewer bytes than the ask's fixed part promises — a length
    /// prefix, or the id a prefix said was there.
    Truncated,
    /// An id that is not UTF-8.
    IdUtf8,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Truncated => {
                f.write_str("fuse request is shorter than it promises")
            }
            RequestError::IdUtf8 => f.write_str("fuse mount id is not utf-8"),
        }
    }
}

impl error::Error for RequestError {}

/// A fuse ask that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestEncodeError {
    /// An id of more bytes than a two-byte length prefix can say,
    /// carrying how many there were. Only [`write`](super::write)
    /// prefixes its id; a read takes any length.
    IdLength(usize),
}

impl fmt::Display for RequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestEncodeError::IdLength(len) => {
                write!(f, "fuse mount id is {len} bytes, more than 65535")
            }
        }
    }
}

impl error::Error for RequestEncodeError {}

/// A fuse answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseError {
    /// No bytes at all, so not even a kind.
    Empty,
    /// A kind this answer does not define.
    UnknownKind(u8),
    /// An error message that is not UTF-8.
    MessageUtf8,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseError::Empty => f.write_str("fuse response is empty"),
            ResponseError::UnknownKind(kind) => {
                write!(f, "unknown fuse response kind {kind}")
            }
            ResponseError::MessageUtf8 => {
                f.write_str("fuse error message is not utf-8")
            }
        }
    }
}

impl error::Error for ResponseError {}
