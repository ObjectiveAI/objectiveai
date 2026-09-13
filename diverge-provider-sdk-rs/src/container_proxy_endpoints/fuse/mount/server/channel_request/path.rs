//! What an ask names when a path is all of it.

use std::str;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// One entry of the mount: its path relative to the mount root —
/// empty for a file mount, and for a directory mount's root.
///
/// ```text
/// [path: utf8…]
/// ```
///
/// The path runs to the end of the payload, so it needs no prefix and
/// takes any length. This is the whole ask for
/// [`Read`](super::Frame::Read), [`List`](super::Frame::List),
/// [`Remove`](super::Frame::Remove), [`Mkdir`](super::Frame::Mkdir) and
/// [`Stat`](super::Frame::Stat), each of which is answered by its own
/// frame. The mount itself is the scope the ask rides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Path<'a> {
    /// The entry's path inside the mount, `/`-separated, no leading
    /// slash; empty is the mount itself.
    pub path: &'a str,
}

impl Encode for Path<'_> {
    /// [`Infallible`](std::convert::Infallible): the path's bytes are
    /// already bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Path<'a> {
    /// Decode from the bytes after the ask's tag. The path borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        Ok(Path {
            path: str::from_utf8(bytes).map_err(|_| FrameError::PathUtf8)?,
        })
    }
}
