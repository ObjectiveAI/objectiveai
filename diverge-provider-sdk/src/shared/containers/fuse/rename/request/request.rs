//! Rename an entry of a mounted directory.

use super::super::super::{RequestEncodeError, RequestError, prefixed};
use crate::encode::{Encode, Writer};

/// Move an entry within one mount, by the mount's id and the two
/// paths. Answered with one [`Ack`](super::super::super::ack::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][from_len: u16 BE][from: utf8…][to: utf8…]
/// ```
///
/// Neither path is empty: the root is never moved. A file at `to` is
/// replaced whole — a program that saves by writing a temporary and
/// renaming it over the real file is doing exactly this — and a
/// directory at `to` is the caller's to refuse. The proxy sends no
/// rename across mounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// Where the entry is.
    pub from: &'a str,
    /// Where it goes.
    pub to: &'a str,
}

impl Encode for Request<'_> {
    /// Two ways to fail: an id or a source path longer than its
    /// prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        prefixed::put(out, self.from.as_bytes()).map_err(RequestEncodeError::PathLength)?;
        out.extend_from_slice(self.to.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. Everything borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (from, to) = prefixed::take(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            from: std::str::from_utf8(from).map_err(|_| RequestError::PathUtf8)?,
            to: std::str::from_utf8(to).map_err(|_| RequestError::PathUtf8)?,
        })
    }
}
