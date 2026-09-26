//! Read a piece of a mounted file.

use super::super::super::{RequestEncodeError, RequestError, prefixed};
use crate::encode::{Encode, Writer};

/// Read at most `length` bytes of a file of a mount from `offset`,
/// by the mount's id and the file's path in it. Answered with one
/// [`read::response::Frame`](super::super::response::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][path_len: u16 BE][path: utf8…][offset: u64 BE][length: u32 BE]
/// ```
///
/// Fixed fields follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. The piece is what the
/// kernel asked the mount for — one `read(2)`, one ask — and the
/// answer is at most `length` bytes: fewer at the end of the file,
/// and none at or past it. Nothing is read ahead, and nothing is
/// kept: the next piece is the next ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// Where the piece starts, in bytes from the file's start.
    pub offset: u64,
    /// How many bytes at most.
    pub length: u32,
}

impl Encode for Request<'_> {
    /// Two ways to fail: an id or a path longer than its prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        prefixed::put(out, self.path.as_bytes()).map_err(RequestEncodeError::PathLength)?;
        out.extend_from_slice(&self.offset.to_be_bytes());
        out.extend_from_slice(&self.length.to_be_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id and the path
    /// borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (path, rest) = prefixed::take(rest)?;
        let (offset, length) = super::super::super::piece::decode(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
            offset,
            length,
        })
    }
}
