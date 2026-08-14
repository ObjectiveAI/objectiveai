//! A write's content, arriving.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A piece of the file being written.
///
/// Bytes and nothing else — no tag, because there is nothing to
/// discriminate. A channel opened by a
/// [`bytes::Request`](super::Request) carries one thing, and the
/// provider knew what it was when it asked.
///
/// # The end, and the giving up
///
/// The content ends when the channel finishes. There is no terminator
/// in the payload because the frame layer already has one, and a
/// second would be two signals for one fact.
///
/// A client that decides mid-stream not to go through with the write
/// — because the read it was piping came back
/// [`Corrupted`](crate::shared::container::read::response::Frame::Corrupted),
/// say — simply never finishes. The provider sees content stop without
/// an end, discards the temporary, and the destination is left
/// untouched. Abandonment is the cancel, and it needs no frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    pub &'a [u8],
);

impl Encode for Frame<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// [`Infallible`]: there is nothing to get wrong about a slice
    /// that is already the answer.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Frame(bytes))
    }
}
