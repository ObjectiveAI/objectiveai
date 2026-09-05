//! Remove a key. Answered `Ok` whether or not it existed — the state

use std::convert::Infallible;

use super::RequestError;
use crate::encode::{Encode, Writer};

/// Remove a key. Answered `Ok` whether or not it existed — the state
/// asked for is the state that results.
///
/// ```text
/// [key: utf8…]
/// ```
///
/// The key is the whole payload: nothing follows it, so nothing
/// delimits it. Kind `7` on `/requests`, answered on
/// `/vault/delete/{channel}` with one [`Done`](super::Done).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Delete<'a> {
    /// The key.
    pub key: &'a str,
}

impl Encode for Delete<'_> {
    /// [`Infallible`]: bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.key.as_bytes());
        Ok(())
    }
}

impl<'a> Delete<'a> {
    /// Decode from the bytes after the ask's kind. The key borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        std::str::from_utf8(bytes)
            .map(|key| Delete { key })
            .map_err(|_| RequestError::KeyUtf8)
    }
}
