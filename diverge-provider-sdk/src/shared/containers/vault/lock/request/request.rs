//! Hold a key's lock.

use std::convert::Infallible;

use super::super::super::RequestError;
use crate::encode::{Encode, Writer};

/// Hold a key's lock for a while, waiting for it. Answered `Ok` once
/// held — see [the module](super::super::super) for whose the lock is, how the TTL
/// is refreshed, and how it ends.
///
/// ```text
/// [ttl: u32 BE seconds][key: utf8…]
/// ```
///
/// The TTL leads so the key can be the rest. Answered with one
/// [`response::Frame`](super::super::response::Frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// The key.
    pub key: &'a str,
    /// How long the lock is held from the grant, in seconds, unless
    /// refreshed by another `Lock` or released by an
    /// [`unlock`](super::super::super::unlock). `0` is refused.
    pub ttl: u32,
}

/// The bytes the TTL occupies.
const TTL_LEN: usize = 4;

impl Encode for Request<'_> {
    /// [`Infallible`]: four known bytes and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(&self.ttl.to_be_bytes());
        out.extend_from_slice(self.key.as_bytes());
        Ok(())
    }
}

impl<'a> Request<'a> {
    /// Decode from the bytes after the ask's kind. The key borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, RequestError> {
        let ttl: &[u8; TTL_LEN] = bytes
            .get(..TTL_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(RequestError::Truncated)?;
        let key = std::str::from_utf8(&bytes[TTL_LEN..])
            .map_err(|_| RequestError::KeyUtf8)?;
        Ok(Request {
            key,
            ttl: u32::from_be_bytes(*ttl),
        })
    }
}
