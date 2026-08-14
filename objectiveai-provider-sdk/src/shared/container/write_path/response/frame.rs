//! What a response frame carries on a write path channel.

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
/// | a [`Frame`], then a finish | the file is at the path |
/// | a finish, and nothing before it | it is not, and nothing partial is |
/// | nothing | the provider died mid-write; the destination is unknowable from here |
///
/// # It still spends a byte
///
/// A payload of zero bytes would carry the same information today and
/// cost a wire break tomorrow. Failure has no shape yet — a provider
/// says so by finishing without this — and when it gets one it will be
/// another tag value, which is only additive if there is a tag to add
/// to. Nothing here has to change when that happens.
///
/// The same reason
/// [`list`](crate::endpoints::filesystem::list::client::request::Frame)
/// spends a byte on a request with no fields.
///
/// # What a partial write leaves behind
///
/// Nothing at the destination. A provider writes to a temporary in the
/// destination's own directory and renames it into place, so the path
/// holds the old file, then nothing, then the new one — never a prefix
/// of the new one. That holds whether this frame arrives or not.
///
/// Where space is too tight for both copies, unlinking the old one
/// first frees exactly what the new one needs. That trades the old
/// contents away on failure, which is why it is worth doing only after
/// the ordinary attempt returns `ENOSPC` rather than up front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says the file landed.
const WRITTEN: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[WRITTEN]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&WRITTEN) => Ok(Frame),
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
            FrameError::Empty => f.write_str("write result frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown write result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
