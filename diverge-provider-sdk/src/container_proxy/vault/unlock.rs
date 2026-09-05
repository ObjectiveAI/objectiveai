//! Release a key's lock early. Answered `Ok`, or `Error` when the

use std::convert::Infallible;

use super::RequestError;
use crate::encode::{Encode, Writer};

/// Release a key's lock early. Answered `Ok`, or `Error` when the
/// asking container does not hold it.
///
/// ```text
/// [key: utf8…]
/// ```
///
/// The key is the whole payload: nothing follows it, so nothing
/// delimits it. Kind `9` on `/requests`, answered on
/// `/vault/unlock/{channel}` with one [`Done`](super::Done).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Unlock<'a> {
    /// The key.
    pub key: &'a str,
}

impl Encode for Unlock<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.key.as_bytes());
        Ok(())
    }
}

impl<'a> Unlock<'a> {
    /// Decode from the bytes after the ask's kind. The key borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        std::str::from_utf8(bytes)
            .map(|key| Unlock { key })
            .map_err(|_| RequestError::KeyUtf8)
    }
}
