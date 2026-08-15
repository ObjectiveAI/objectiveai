//! What a response frame carries on a transfer channel.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The file landed.
///
/// One of these, then the channel finishes. It carries nothing,
/// because saying so IS the whole message.
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | a [`Frame`], then a finish | the file is at the destination |
/// | a finish, and nothing before it | it is not, and nothing partial is |
/// | nothing | the provider died mid-transfer; the destination is unknowable from here |
///
/// Which is [`write_path`](crate::shared::container::write_path)'s
/// shape, and a separate type rather than an alias of it. They mean
/// the same thing today and will not once failures have one: a
/// transfer can fail for reasons a write has no room for — a source
/// that is not there, two containers a provider will not put in
/// contact — and a shared type would have to carry both sets forever.
///
/// # A silent tear is possible here
///
/// A [`read`](crate::shared::container::read) reports a source that
/// moved underneath it. This has nowhere to say so. A provider can
/// still detect it — the same `fstat` before and after — but until
/// failures have a shape there is no frame that means "copied, and the
/// source changed while I did".
///
/// So a transfer that returns this frame promises the destination
/// exists and holds one whole file. It does not promise that file ever
/// existed at the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says the file landed.
const TRANSFERRED: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TRANSFERRED]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&TRANSFERRED) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A transfer result that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a provider reporting a failure will send, once
    /// failures have a shape. Until then it is a peer that disagrees
    /// about the protocol.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("transfer result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown transfer result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
