//! What a client's request frame carries for a version request.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask a provider what version it is.
///
/// Carries nothing. The tag is the whole request — there is no
/// parameter because there is nothing to ask about beyond the asking,
/// and a field that narrowed the question would be a field a provider
/// had to interpret before it could say the one thing it knows.
///
/// # A struct, and still a tag byte
///
/// The byte is what every scope-opening request spends to name itself,
/// so this is not a cost this request chose. It is what makes an empty
/// payload a REQUEST rather than an empty payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 12;

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

/// A version request that could not be read.
///
/// Anything after the tag is ignored rather than refused. There is
/// nothing defined to follow one, so bytes that do mean a peer knows
/// something this version does not — and leaving room for it is
/// cheaper than rejecting a request whose whole meaning already
/// arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("version request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected version request tag {TAG}, found {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
