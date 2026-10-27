//! What a client's request frame carries for an image listing.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask a provider which images the caller has built on it.
///
/// A unit struct, because the question has no parameters. There is
/// nothing to narrow: the answer is every image the caller built and
/// has not deleted, and a caller cannot ask about one it has not been
/// told the name of.
///
/// # A frame that is only a tag
///
/// It still has to go on the wire, because
/// [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// carries one type of frame and the payload's leading byte is what
/// says which request it is. So this encodes to exactly one byte and
/// decodes by reading exactly one byte — the tag and nothing after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::provider::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 16;

impl Encode for Frame {
    /// [`Infallible`]: writing one known byte has no failure mode.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Only the tag can be wrong, because only the tag is read.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&TAG) => Ok(Frame),
            Some(&tag) => Err(FrameError::UnexpectedTag(tag)),
            None => Err(FrameError::Empty),
        }
    }
}

/// An image listing request that could not be read.
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
            FrameError::Empty => f.write_str("image listing request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected image listing request tag {TAG}, found {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
