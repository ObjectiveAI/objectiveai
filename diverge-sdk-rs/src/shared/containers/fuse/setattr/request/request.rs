//! Set a mounted entry's attributes.

use super::super::super::{Attrs, RequestEncodeError, RequestError, prefixed};
use crate::wire::encode::{Encode, Writer};

/// Set some of an entry's attributes, by the mount's id and the
/// entry's path in it. Answered with one
/// [`Ack`](super::super::super::ack::Frame).
///
/// ```text
/// [id_len: u16 BE][id: utf8…][attrs: 29 bytes][path: utf8…]
/// ```
///
/// The attributes precede the path, so the path runs to the end and
/// needs no prefix. On a file mount the path is empty. What is not
/// set is left as it is; what is set is set as the caller's storage
/// sets it, and a caller that cannot — one with no owners to speak
/// of — answers ok and keeps what it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
    /// The entry's path inside the mount; empty for a file mount, and
    /// for a directory mount's root.
    pub path: &'a str,
    /// Which attributes, and to what. See [`Attrs`].
    pub attrs: Attrs,
}

impl Encode for Request<'_> {
    /// One way to fail: an id longer than its prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        prefixed::put(out, self.id.as_bytes()).map_err(RequestEncodeError::IdLength)?;
        self.attrs.encode(out);
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id and the path
    /// borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let (id, rest) = prefixed::take(bytes)?;
        let (attrs, path) = Attrs::decode(rest)?;
        Ok(Request {
            id: std::str::from_utf8(id).map_err(|_| RequestError::IdUtf8)?,
            path: std::str::from_utf8(path).map_err(|_| RequestError::PathUtf8)?,
            attrs,
        })
    }
}
