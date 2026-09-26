//! Write a piece of a mounted file.

use std::str;

use super::{FrameEncodeError, FrameError, prefixed};
use crate::wire::encode::{Encode, Writer};

/// Write a piece of a file of the mount in place at `offset`, by its
/// path. Answered with one
/// [`Ack`](crate::shared::containers::fuse::ack::Frame).
///
/// ```text
/// [path_len: u16 BE][path: utf8…][offset: u64 BE][bytes…]
/// ```
///
/// The bytes follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. One `write(2)`, one ask:
/// a file that does not exist yet is made by this, one shorter than
/// the offset is extended to it, and the bytes at and after the
/// offset are replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Write<'a> {
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// Where the piece lands, in bytes from the file's start.
    pub offset: u64,
    /// The piece, verbatim. Empty writes nothing and makes the file.
    pub bytes: &'a [u8],
}

impl Encode for Write<'_> {
    /// One way to fail: a path longer than its prefix holds.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        prefixed::put(out, self.path.as_bytes())?;
        out.extend_from_slice(&self.offset.to_be_bytes());
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Write<'a> {
    /// Decode from the bytes after the ask's tag. The path and the
    /// bytes borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (path, rest) = prefixed::take(bytes)?;
        let offset: [u8; 8] = rest.get(..8).and_then(|head| head.try_into().ok()).ok_or(FrameError::Truncated)?;
        Ok(Write {
            path: str::from_utf8(path).map_err(|_| FrameError::PathUtf8)?,
            offset: u64::from_be_bytes(offset),
            bytes: &rest[8..],
        })
    }
}
