//! What a server's response frame carries for a volume edit.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The volume reserves what was asked for.
///
/// One of these on channel `0`, then the scope finishes. It carries
/// nothing, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | a [`Frame`], then a finish | a listing will report the new size |
/// | a finish, and nothing before it | it will report the old one |
/// | nothing | the connection died; which of the two is unknowable from here |
///
/// # It still spends a byte
///
/// A payload of zero bytes would carry the same information today and
/// cost a wire break tomorrow. Failure has no shape yet — a provider
/// says so by finishing without this — and when it gets one it will be
/// another tag value, which is only additive if there is a tag to add
/// to.
///
/// The same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says the size is what was asked for.
const EDITED: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[EDITED]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&EDITED) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A volume edit result that could not be read.
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
                f.write_str("volume edit result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume edit result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
