//! Truncate a mounted file.

use super::super::super::{RequestEncodeError, RequestError, prefixed};
use crate::wire::encode::{Encode, Writer};

/// Set a file of a mount to `size` bytes, by the mount's id and the
/// file's path in it. Answered with one
/// [`Ack`](super::super::super::ack::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][size: u64 BE][path: utf8…]
/// ```
///
/// The size precedes the path, so the path runs to the end and needs
/// no prefix. On a file mount the path is empty. A file longer than
/// `size` loses its tail; one shorter gains zeros; a file that does
/// not exist yet is made, of that length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// The length the file has after, in bytes.
    pub size: u64,
}

impl Encode for Request<'_> {
    /// One way to fail: an id longer than its prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        out.extend_from_slice(&self.size.to_be_bytes());
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id and the path
    /// borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (size, path) = super::super::super::piece::offset(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
            size,
        })
    }
}
