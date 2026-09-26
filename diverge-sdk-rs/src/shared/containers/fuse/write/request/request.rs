//! Write a mounted file.

use super::super::super::{RequestEncodeError, RequestError, prefixed};
use crate::wire::encode::{Encode, Writer};

/// Write a piece of a file of a mount in place at `offset`, by the
/// mount's id and the file's path in it. Answered with one
/// [`Ack`](super::super::super::ack::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][path_len: u16 BE][path: utf8…][offset: u64 BE][bytes…]
/// ```
///
/// The bytes follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. The piece is what the
/// kernel handed the mount — one `write(2)`, one ask, at most
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE) — and it lands where it says: a
/// file that does not exist yet is made by this, one shorter than
/// the offset is extended with zeros to it, and the bytes at and
/// after the offset are replaced by these. Nothing is held back for
/// a close: the file changes as the pieces arrive, and a
/// [`truncate`](super::super::super::truncate) is how it gets shorter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// Where the piece lands, in bytes from the file's start.
    pub offset: u64,
    /// The piece, verbatim. Empty writes nothing and makes the file.
    pub bytes: &'a [u8],
}

impl Encode for Request<'_> {
    /// Two ways to fail: an id or a path longer than its prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        prefixed::put(out, self.path.as_bytes()).map_err(RequestEncodeError::PathLength)?;
        out.extend_from_slice(&self.offset.to_be_bytes());
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id, the path
    /// and the bytes borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (path, rest) = prefixed::take(rest)?;
        let (offset, bytes) = super::super::super::piece::offset(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
            offset,
            bytes,
        })
    }
}
