//! Rename an entry of a mounted directory.

use std::str;

use super::{FrameEncodeError, FrameError, prefixed};
use crate::encode::{Encode, Writer};

/// Move an entry within the mount, by its two paths. Answered with
/// one [`Ack`](crate::shared::containers::fuse::ack::Frame).
///
/// ```text
/// [from_len: u16 BE][from: utf8…][to: utf8…]
/// ```
///
/// Neither path is empty: the root is never moved. A file at `to` is
/// replaced whole — a program that saves by writing a temporary and
/// renaming it over the real file is doing exactly this — and a
/// directory at `to` is the caller's to refuse. The proxy sends no
/// rename across mounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rename<'a> {
    /// Where the entry is.
    pub from: &'a str,
    /// Where it goes.
    pub to: &'a str,
}

impl Encode for Rename<'_> {
    /// One way to fail: a source path longer than its prefix holds.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        prefixed::put(out, self.from.as_bytes())?;
        out.extend_from_slice(self.to.as_bytes());
        Ok(())
    }
}

impl<'a> Rename<'a> {
    /// Decode from the bytes after the ask's tag. Both paths borrow
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (from, to) = prefixed::take(bytes)?;
        Ok(Rename {
            from: str::from_utf8(from).map_err(|_| FrameError::PathUtf8)?,
            to: str::from_utf8(to).map_err(|_| FrameError::PathUtf8)?,
        })
    }
}
