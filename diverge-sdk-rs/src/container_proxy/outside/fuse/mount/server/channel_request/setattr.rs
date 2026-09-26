//! Set a mounted entry's attributes.

use std::str;

use super::FrameError;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::fuse::Attrs;

/// Set some of an entry's attributes, by its path. Answered with one
/// [`Ack`](crate::shared::containers::fuse::ack::Frame).
///
/// ```text
/// [attrs: 29 bytes][path: utf8…]
/// ```
///
/// The attributes precede the path, so the path runs to the end and
/// needs no prefix — see [`Attrs`] for the twenty-nine bytes. On a
/// file mount the path is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Setattr<'a> {
    /// The entry's path inside the mount; empty for a file mount, and
    /// for a directory mount's root.
    pub path: &'a str,
    /// Which attributes, and to what.
    pub attrs: Attrs,
}

impl Encode for Setattr<'_> {
    /// [`Infallible`](std::convert::Infallible): fixed bytes and a
    /// path's own.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        self.attrs.encode(out);
        out.extend_from_slice(self.path.as_bytes());
        Ok(())
    }
}

impl<'a> Setattr<'a> {
    /// Decode from the bytes after the ask's tag. The path borrows
    /// from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (attrs, path) = Attrs::decode(bytes).map_err(|_| FrameError::Truncated)?;
        Ok(Setattr {
            path: str::from_utf8(path).map_err(|_| FrameError::PathUtf8)?,
            attrs,
        })
    }
}
