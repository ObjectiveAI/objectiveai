//! What a client's request frame carries for a volume list.

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};

/// The provider's own request, which carries nothing, behind the
/// daemon's tag.
///
/// One byte, the tag, and that is the whole request — as the
/// provider's [`diverge_provider_sdk::endpoints::volumes::list::client::request::Frame`] is, the question
/// having no parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](diverge_provider_sdk::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 5;

/// The tag, and nothing after it.
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

    /// Whatever follows the tag is ignored: nothing is defined to, and
    /// leaving room for it is cheaper than refusing a request whose
    /// whole meaning already arrived.
    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        Ok(Frame)
    }
}

/// A volume list request that could not be read.
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
            FrameError::Empty => f.write_str("volume list request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected volume list request tag {TAG}, found {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
