//! Write a key.

use super::{RequestEncodeError, RequestError};
use crate::encode::{Encode, Writer};

/// Write a key, creating or replacing it. Answered `Ok`.
///
/// ```text
/// [key_len: u16 BE][key: utf8…][value…]
/// ```
///
/// The one operation whose key has a length prefix, because a value
/// follows it and nothing else delimits the two. Kind `6` on
/// `/requests`, answered on `/vault/set/{channel}` with one
/// [`Done`](super::Done).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Set<'a> {
    /// The key.
    pub key: &'a str,
    /// The value, verbatim. Empty is a value.
    pub value: &'a [u8],
}

/// The bytes the key's length occupies.
const KEY_LEN: usize = 2;

impl Encode for Set<'_> {
    /// One way to fail: a key longer than the length prefix holds.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        let key = self.key.as_bytes();
        let len = u16::try_from(key.len())
            .map_err(|_| RequestEncodeError::KeyLength(key.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(key);
        out.extend_from_slice(self.value);
        Ok(())
    }
}

impl<'a> Set<'a> {
    /// Decode from the bytes after the ask's kind. The key and the
    /// value borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let len: &[u8; KEY_LEN] = bytes
            .get(..KEY_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(RequestError::Truncated)?;
        let len = usize::from(u16::from_be_bytes(*len));
        let rest = &bytes[KEY_LEN..];
        let key = rest.get(..len).ok_or(RequestError::Truncated)?;
        let key = std::str::from_utf8(key).map_err(|_| RequestError::KeyUtf8)?;
        Ok(Set {
            key,
            value: &rest[len..],
        })
    }
}
