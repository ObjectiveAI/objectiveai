//! Read a piece of a mounted file.

use std::str;

use super::{FrameEncodeError, FrameError, prefixed};
use crate::encode::{Encode, Writer};

/// Read at most `length` bytes of a file of the mount from `offset`,
/// by its path. Answered with one
/// [`read::response::Frame`](crate::shared::containers::fuse::read::response::Frame).
///
/// ```text
/// [path_len: u16 BE][path: utf8…][offset: u64 BE][length: u32 BE]
/// ```
///
/// Fixed fields follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. One `read(2)`, one ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Read<'a> {
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// Where the piece starts, in bytes from the file's start.
    pub offset: u64,
    /// How many bytes at most.
    pub length: u32,
}

impl Encode for Read<'_> {
    /// One way to fail: a path longer than its prefix holds.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        prefixed::put(out, self.path.as_bytes())?;
        out.extend_from_slice(&self.offset.to_be_bytes());
        out.extend_from_slice(&self.length.to_be_bytes());
        Ok(())
    }
}

impl<'a> Read<'a> {
    /// Decode from the bytes after the ask's tag. The path borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (path, rest) = prefixed::take(bytes)?;
        let offset: [u8; 8] = rest.get(..8).and_then(|head| head.try_into().ok()).ok_or(FrameError::Truncated)?;
        let length: [u8; 4] = rest.get(8..12).and_then(|head| head.try_into().ok()).ok_or(FrameError::Truncated)?;
        Ok(Read {
            path: str::from_utf8(path).map_err(|_| FrameError::PathUtf8)?,
            offset: u64::from_be_bytes(offset),
            length: u32::from_be_bytes(length),
        })
    }
}
