//! Write a mounted file.

use std::str;

use super::{FrameEncodeError, FrameError, prefixed};
use crate::encode::{Encode, Writer};

/// Write a file of the mount, whole, by its path. Answered with one
/// [`Ack`](crate::shared::containers::fuse::ack::Frame).
///
/// ```text
/// [path_len: u16 BE][path: utf8…][bytes…]
/// ```
///
/// The bytes follow the path, so the path carries a length prefix
/// here. On a file mount the path is empty. A file that does not
/// exist yet is made by this; one that does is replaced whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Write<'a> {
    /// The file's path inside the mount; empty for a file mount.
    pub path: &'a str,
    /// The file, whole, verbatim. Empty is a file.
    pub bytes: &'a [u8],
}

impl Encode for Write<'_> {
    /// One way to fail: a path longer than its prefix holds.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        prefixed::put(out, self.path.as_bytes())?;
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Write<'a> {
    /// Decode from the bytes after the ask's tag. The path and the
    /// bytes borrow from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (path, bytes) = prefixed::take(bytes)?;
        Ok(Write {
            path: str::from_utf8(path).map_err(|_| FrameError::PathUtf8)?,
            bytes,
        })
    }
}
