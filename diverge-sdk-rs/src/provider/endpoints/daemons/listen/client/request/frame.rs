//! What a client's request frame carries for a listen.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask a provider for every message sent to the caller's identity.
///
/// Carries nothing. The tag is the whole request: the receiver is the
/// identity the connection was authorized under, and nothing else
/// names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// This frame's tag among the scope-opening requests. See the table
/// in [`endpoints`](crate::provider::endpoints).
const TAG: u8 = 26;

/// One byte, and no serialization. There is nothing to serialize.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        Ok(Frame)
    }
}

/// A listen request that could not be read. Anything after the tag is
/// ignored rather than refused, as a daemons list request's is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("listen request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "listen request frame has tag {tag}, expected {TAG}"),
        }
    }
}

impl std::error::Error for FrameError {}
