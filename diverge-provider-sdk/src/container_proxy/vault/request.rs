//! What the container asks of the vault.

use std::error;
use std::fmt;

use crate::encode::{Encode, Writer};

/// One request against the caller's vault.
///
/// ```text
/// [kind: u8][key_len: u16 BE][key: utf8…][value…]
/// ```
///
/// Every kind names a key; only [`Set`](Self::Set) carries anything
/// after it. The key's length is prefixed because a value follows it
/// on one kind and nothing delimits the two otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// Kind `0`. Read the key. Answered with
    /// [`Value`](super::response::Response::Value) or
    /// [`Missing`](super::response::Response::Missing).
    Get {
        /// The key.
        key: &'a str,
    },
    /// Kind `1`. Write the key, creating or replacing it. Answered
    /// with [`Ok`](super::response::Response::Ok).
    Set {
        /// The key.
        key: &'a str,
        /// The value, verbatim. Empty is a value.
        value: &'a [u8],
    },
    /// Kind `2`. Remove the key. Answered with
    /// [`Ok`](super::response::Response::Ok) whether or not it
    /// existed — the state asked for is the state that results.
    Delete {
        /// The key.
        key: &'a str,
    },
    /// Kind `3`. Hold the key's lock, waiting for it. Answered with
    /// [`Ok`](super::response::Response::Ok) once held — see [the
    /// module](super) for whose the lock is and how it ends.
    Lock {
        /// The key.
        key: &'a str,
    },
    /// Kind `4`. Release the key's lock. Answered with
    /// [`Ok`](super::response::Response::Ok), or
    /// [`Error`](super::response::Response::Error) when this
    /// connection does not hold it.
    Unlock {
        /// The key.
        key: &'a str,
    },
}

/// The bytes a key's length occupies.
const KEY_LEN: usize = 2;

impl Request<'_> {
    fn kind(&self) -> u8 {
        match self {
            Request::Get { .. } => 0,
            Request::Set { .. } => 1,
            Request::Delete { .. } => 2,
            Request::Lock { .. } => 3,
            Request::Unlock { .. } => 4,
        }
    }

    fn key(&self) -> &str {
        match self {
            Request::Get { key }
            | Request::Set { key, .. }
            | Request::Delete { key }
            | Request::Lock { key }
            | Request::Unlock { key } => key,
        }
    }
}

impl Encode for Request<'_> {
    /// One way to fail: a key longer than the length prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        let key = self.key().as_bytes();
        let len = u16::try_from(key.len())
            .map_err(|_| RequestEncodeError::KeyLength(key.len()))?;
        out.extend_from_slice(&[self.kind()]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(key);
        if let Request::Set { value, .. } = self {
            out.extend_from_slice(value);
        }
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode one request from the bytes after the channel byte.
    /// The key and the value borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (kind, rest) = bytes.split_first().ok_or(RequestError::Truncated)?;
        let len: &[u8; KEY_LEN] = rest
            .get(..KEY_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(RequestError::Truncated)?;
        let len = usize::from(u16::from_be_bytes(*len));
        let rest = &rest[KEY_LEN..];
        let key = rest.get(..len).ok_or(RequestError::Truncated)?;
        let key = std::str::from_utf8(key).map_err(|_| RequestError::KeyUtf8)?;
        let value = &rest[len..];
        match *kind {
            0 => Ok(Request::Get { key }),
            1 => Ok(Request::Set { key, value }),
            2 => Ok(Request::Delete { key }),
            3 => Ok(Request::Lock { key }),
            4 => Ok(Request::Unlock { key }),
            other => Err(RequestError::UnknownKind(other)),
        }
    }
}

/// A vault request that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestEncodeError {
    /// A key of more bytes than the two-byte length prefix can say,
    /// carrying how many there were.
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

/// A vault request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// Fewer bytes than the kind, the length and the key it promises.
    Truncated,
    /// A kind this path does not define.
    UnknownKind(u8),
    /// A key that is not UTF-8.
    KeyUtf8,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Truncated => {
                f.write_str("vault request is shorter than its key promises")
            }
            RequestError::UnknownKind(kind) => {
                write!(f, "unknown vault request kind {kind}")
            }
            RequestError::KeyUtf8 => f.write_str("vault key is not utf-8"),
        }
    }
}

impl error::Error for RequestError {}
