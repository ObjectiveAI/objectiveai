//! Read a key. Answered with the value, or that there is none.

use std::convert::Infallible;

use super::super::super::RequestError;
use crate::encode::{Encode, Writer};

/// Read a key. Answered with the value, or that there is none.
///
/// ```text
/// [key: utf8…]
/// ```
///
/// The key is the whole payload: nothing follows it, so nothing
/// delimits it. Kind `5` on `/requests`, answered on
/// `/vault/get/{channel}` with one
/// [`response::Frame`](super::super::response::Frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The key.
    pub key: &'a str,
}

impl Encode for Request<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.key.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The key borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        std::str::from_utf8(bytes)
            .map(|key| Request { key })
            .map_err(|_| RequestError::KeyUtf8)
    }
}
