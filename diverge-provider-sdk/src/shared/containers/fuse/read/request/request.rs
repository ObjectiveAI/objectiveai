//! Read a mounted file. Answered with its bytes, or that there are
//! none.

use std::convert::Infallible;

use super::super::super::RequestError;
use crate::encode::{Encode, Writer};

/// Read a mounted file, by the id the caller gave its mount.
/// Answered with the bytes, or that there are none.
///
/// ```text
/// [id: utf8…]
/// ```
///
/// The id is the whole payload: nothing follows it, so nothing
/// delimits it. Answered with one
/// [`response::Frame`](super::super::response::Frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The mount's id.
    pub id: &'a str,
}

impl Encode for Request<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.id.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The id borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        std::str::from_utf8(bytes)
            .map(|id| Request { id })
            .map_err(|_| RequestError::IdUtf8)
    }
}
