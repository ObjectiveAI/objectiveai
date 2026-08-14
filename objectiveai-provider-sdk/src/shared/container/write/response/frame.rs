//! What a response frame carries on a write channel.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Whether the file landed.
///
/// One of these, then the channel finishes. Unlike a
/// [`read`](crate::shared::container::read), where bodies precede the
/// finish and the finish alone can mean success, a write channel
/// carries no bodies at all — so a bare finish would be the only
/// signal and could not tell "it is written" from "I stopped trying".
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | [`Written`](Self::Written), then a finish | the file is at the path |
/// | [`Failed`](Self::Failed), then a finish | it is not, and nothing partial is |
/// | nothing | the provider died mid-write; the destination is unknowable from here |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// The file is at the path. Tag `0`.
    ///
    /// Which means the rename completed. A provider that reports this
    /// has already put the whole file in place — there is no state in
    /// which some of it is there.
    Written,
    /// It is not. Tag `1`.
    ///
    /// The destination holds whatever it held before, or nothing if
    /// the provider had to free that space to make room. It never
    /// holds part of what was being written.
    ///
    /// No reason. A caller's response to any of them is the same —
    /// stop, or try again later — and enumerating filesystem errors
    /// here would mean this specification tracking a kernel's.
    Failed,
}

/// Tag for [`Frame::Written`].
const WRITTEN: u8 = 0;

/// Tag for [`Frame::Failed`].
const FAILED: u8 = 1;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[match self {
            Frame::Written => WRITTEN,
            Frame::Failed => FAILED,
        }]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&WRITTEN) => Ok(Frame::Written),
            Some(&FAILED) => Ok(Frame::Failed),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A write result that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither [`Frame::Written`] nor
    /// [`Frame::Failed`].
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("write result frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown write result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
