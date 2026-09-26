//! Truncate a mounted file.

use std::str;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// Set a file of the mount to `size` bytes, by its path. Answered
/// with one [`Ack`](crate::shared::containers::fuse::ack::Frame).
///
/// ```text
/// [size: u64 BE][path: utf8…]
/// ```
///
/// The size precedes the path, so the path runs to the end and needs
/// no prefix. On a file mount the path is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Truncate<'a> {
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// The length the file has after, in bytes.
    pub size: u64,
}

impl Encode for Truncate<'_> {
    /// [`Infallible`](std::convert::Infallible): eight bytes and a
    /// path's own.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.size.to_be_bytes());
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Truncate<'a> {
    /// Decode from the bytes after the ask's tag. The path borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let size: [u8; 8] = bytes.get(..8).and_then(|head| head.try_into().ok()).ok_or(FrameError::Truncated)?;
        Ok(Truncate {
            path: str::from_utf8(&bytes[8..]).map_err(|_| FrameError::PathUtf8)?,
            size: u64::from_be_bytes(size),
        })
    }
}
