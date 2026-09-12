//! Write a mounted file.

use super::super::super::{RequestEncodeError, RequestError, prefixed};
use crate::encode::{Encode, Writer};

/// Write a file of a mount, whole, by the mount's id and the file's
/// path in it. Answered with one [`Ack`](super::super::super::ack::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][path_len: u16 BE][path: utf8…][bytes…]
/// ```
///
/// The bytes follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. A file that does not
/// exist yet is made by this; one that does is replaced whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// The file, whole, verbatim. Empty is a file.
    pub bytes: &'a [u8],
}

impl Encode for Request<'_> {
    /// Two ways to fail: an id or a path longer than its prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        prefixed::put(out, self.path.as_bytes()).map_err(RequestEncodeError::PathLength)?;
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id, the path
    /// and the bytes borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (path, bytes) = prefixed::take(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
            bytes,
        })
    }
}
