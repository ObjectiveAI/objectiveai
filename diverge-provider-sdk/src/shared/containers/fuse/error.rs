//! Why a fuse ask or answer could not be read or written.

use std::error;
use std::fmt;

/// A fuse ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// Fewer bytes than the ask's fixed part promises — a length
    /// prefix, or the id or path a prefix said was there.
    Truncated,
    /// An id that is not UTF-8.
    IdUtf8,
    /// A path that is not UTF-8.
    PathUtf8,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Truncated => {
                f.write_str("fuse request is shorter than it promises")
            }
            RequestError::IdUtf8 => f.write_str("fuse mount id is not utf-8"),
            RequestError::PathUtf8 => f.write_str("fuse path is not utf-8"),
        }
    }
}

impl error::Error for RequestError {}

/// A fuse ask that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestEncodeError {
    /// An id of more bytes than a two-byte length prefix can say,
    /// carrying how many there were.
    IdLength(usize),
    /// A path of more bytes than a two-byte length prefix can say,
    /// carrying how many there were. Only the asks where something
    /// follows the path prefix it; a path that ends the payload takes
    /// any length.
    PathLength(usize),
}

impl fmt::Display for RequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestEncodeError::IdLength(len) => {
                write!(f, "fuse mount id is {len} bytes, more than 65535")
            }
            RequestEncodeError::PathLength(len) => {
                write!(f, "fuse path is {len} bytes, more than 65535")
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
    /// A kind this answer does not define — of the message, or of an
    /// entry in a listing.
    UnknownKind(u8),
    /// An error message that is not UTF-8.
    MessageUtf8,
    /// A listing shorter than its counts and lengths promise, or a
    /// stat shorter than its nine bytes.
    Truncated,
    /// An entry's name that is not UTF-8.
    NameUtf8,
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
            ResponseError::Truncated => {
                f.write_str("fuse answer is shorter than it promises")
            }
            ResponseError::NameUtf8 => f.write_str("fuse entry name is not utf-8"),
        }
    }
}

impl error::Error for ResponseError {}

/// A fuse answer that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseEncodeError {
    /// An entry's name of more bytes than a two-byte length prefix can
    /// say, carrying how many there were.
    NameLength(usize),
}

impl fmt::Display for ResponseEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseEncodeError::NameLength(len) => {
                write!(f, "fuse entry name is {len} bytes, more than 65535")
            }
        }
    }
}

impl error::Error for ResponseEncodeError {}
