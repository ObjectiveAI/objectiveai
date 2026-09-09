//! Write a mounted file.

use super::super::super::{RequestEncodeError, RequestError};
use crate::encode::{Encode, Writer};

/// Write a mounted file, whole, by the id the caller gave its mount.
/// Answered `Ok`.
///
/// ```text
/// [id_len: u16 BE][id: utf8…][bytes…]
/// ```
///
/// The one operation whose id has a length prefix, because the bytes
/// follow it and nothing else delimits the two. Answered with one
/// [`response::Frame`](super::super::response::Frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The file, whole, verbatim. Empty is a file.
    pub bytes: &'a [u8],
}

/// The bytes the id's length occupies.
const ID_LEN: usize = 2;

impl Encode for Request<'_> {
    /// One way to fail: an id longer than the length prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        let id = self.id.as_bytes();
        let len = u16::try_from(id.len())
            .map_err(|_| RequestEncodeError::IdLength(id.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(id);
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id and the
    /// bytes borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let len: &[u8; ID_LEN] = bytes
            .get(..ID_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(RequestError::Truncated)?;
        let len = usize::from(u16::from_be_bytes(*len));
        let rest = &bytes[ID_LEN..];
        let id = rest.get(..len).ok_or(RequestError::Truncated)?;
        let id = std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?;
        Ok(Request {
            id,
            bytes: &rest[len..],
        })
    }
}
