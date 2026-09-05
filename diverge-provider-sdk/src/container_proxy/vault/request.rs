//! What the container asks of the vault.

use std::error;
use std::fmt;

use crate::encode::{Encode, Writer};

/// One operation against the caller's vault — the payload of a
/// [`Vault`](crate::container_proxy::requests::Request::Vault) ask.
///
/// ```text
/// [op: u8][key_len: u16 BE][key: utf8…][value…]
/// ```
///
/// Every operation names a key; only [`Set`](Self::Set) carries
/// anything after it. The key's length is prefixed because a value
/// follows it on one operation and nothing delimits the two
/// otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Request<'a> {
    /// Op `0`. Read the key. Answered with [`Value`](super::Response::Value)
    /// or [`Missing`](super::Response::Missing).
    Get {
        /// The key.
        key: &'a str,
    },
    /// Op `1`. Write the key, creating or replacing it. Answered with
    /// [`Ok`](super::Response::Ok).
    Set {
        /// The key.
        key: &'a str,
        /// The value, verbatim. Empty is a value.
        value: &'a [u8],
    },
    /// Op `2`. Remove the key. Answered with [`Ok`](super::Response::Ok)
    /// whether or not it existed — the state asked for is the state
    /// that results.
    Delete {
        /// The key.
        key: &'a str,
    },
    /// Op `3`. Hold the key's lock, waiting for it. Answered with
    /// [`Ok`](super::Response::Ok) once held — see [the module](super) for
    /// whose the lock is and how it ends.
    Lock {
        /// The key.
        key: &'a str,
    },
    /// Op `4`. Release the key's lock. Answered with
    /// [`Ok`](super::Response::Ok), or [`Error`](super::Response::Error) when the
    /// asking connection does not hold it.
    Unlock {
        /// The key.
        key: &'a str,
    },
}

/// The bytes a key's length occupies.
const KEY_LEN: usize = 2;

impl Request<'_> {
    fn op(&self) -> u8 {
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
        out.extend_from_slice(&[self.op()]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(key);
        if let Request::Set { value, .. } = self {
            out.extend_from_slice(value);
        }
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode one operation from the bytes after the ask's kind. The
    /// key and the value borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (op, rest) = bytes.split_first().ok_or(RequestError::Truncated)?;
        let len: &[u8; KEY_LEN] = rest
            .get(..KEY_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(RequestError::Truncated)?;
        let len = usize::from(u16::from_be_bytes(*len));
        let rest = &rest[KEY_LEN..];
        let key = rest.get(..len).ok_or(RequestError::Truncated)?;
        let key = std::str::from_utf8(key).map_err(|_| RequestError::KeyUtf8)?;
        let value = &rest[len..];
        match *op {
            0 => Ok(Request::Get { key }),
            1 => Ok(Request::Set { key, value }),
            2 => Ok(Request::Delete { key }),
            3 => Ok(Request::Lock { key }),
            4 => Ok(Request::Unlock { key }),
            other => Err(RequestError::UnknownOp(other)),
        }
    }
}

/// A vault operation that could not be written.
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

/// A vault operation that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestError {
    /// Fewer bytes than the op, the length and the key it promises.
    Truncated,
    /// An op this wire does not define.
    UnknownOp(u8),
    /// A key that is not UTF-8.
    KeyUtf8,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Truncated => {
                f.write_str("vault request is shorter than its key promises")
            }
            RequestError::UnknownOp(op) => {
                write!(f, "unknown vault operation {op}")
            }
            RequestError::KeyUtf8 => f.write_str("vault key is not utf-8"),
        }
    }
}

impl error::Error for RequestError {}
