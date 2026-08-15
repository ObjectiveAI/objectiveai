//! What a server's response frame carries for a volume deletion.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The volume is gone.
///
/// One of these on channel `0`, then the scope finishes. It carries
/// nothing, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | a [`Frame`], then a finish | the volume is gone and a listing will not show it |
/// | a finish, and nothing before it | it is not, and a caller should assume it is intact |
/// | nothing | the connection died; whether it was deleted is unknowable from here |
///
/// # Gone means gone, not emptied
///
/// The volume itself no longer exists. A caller that wanted the space
/// back with the name kept deletes and creates — which is two asks
/// because they are two things, and a provider that emptied one in
/// place would be doing something this frame has no way to distinguish
/// from the other.
///
/// # There is no partial deletion to report
///
/// A provider either finishes destroying the volume or does not report
/// success. What it leaves behind on failure is its own business and
/// not a state a caller can observe: a
/// [`list`](crate::endpoints::volumes::list) shows the volume or it
/// does not, and that is the only answer this protocol offers about
/// whether something exists.
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

/// The tag that says the volume is gone.
const DELETED: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[DELETED]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&DELETED) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A volume deletion result that could not be read.
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
                f.write_str("volume deletion result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume deletion result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
