//! Why a vault ask or answer could not be read or written.

use std::error;
use std::fmt;

/// A vault ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// Fewer bytes than the ask's fixed part promises — a length
    /// prefix, a TTL, or the key a prefix said was there.
    Truncated,
    /// A key that is not UTF-8.
    KeyUtf8,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Truncated => {
                f.write_str("vault request is shorter than it promises")
            }
            RequestError::KeyUtf8 => f.write_str("vault key is not utf-8"),
        }
    }
}

impl error::Error for RequestError {}

/// A vault ask that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestEncodeError {
    /// A key of more bytes than a two-byte length prefix can say,
    /// carrying how many there were. Only [`Set`](super::Set)
    /// prefixes its key; the others take any length.
    KeyLength(usize),
}

impl fmt::Display for RequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestEncodeError::KeyLength(len) => {
                write!(f, "vault key is {len} bytes, more than 65535")
            }
        }
    }
}

impl error::Error for RequestEncodeError {}

/// A vault answer that could not be read.
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
            ResponseError::Empty => f.write_str("vault response is empty"),
            ResponseError::UnknownKind(kind) => {
                write!(f, "unknown vault response kind {kind}")
            }
            ResponseError::MessageUtf8 => {
                f.write_str("vault error message is not utf-8")
            }
        }
    }
}

impl error::Error for ResponseError {}
