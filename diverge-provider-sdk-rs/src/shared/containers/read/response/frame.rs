//! What a response frame carries on a read channel.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A piece of the file.
///
/// Bytes and nothing else — no tag, because there is nothing to
/// discriminate. A read channel carries one kind of traffic from the
/// first byte to the last, and the requester knew what it was when it
/// asked.
///
/// # How it ends
///
/// With a finish, and only with a finish. There is no `Complete` and
/// there is no failure: a read that finished cleanly is one whose
/// channel finished —
/// [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish)
/// says so at the frame layer, and saying it again in the payload
/// would be two signals for one fact.
///
/// # A silent tear is possible here
///
/// A file being read can be written underneath the reader, and the
/// bytes already sent are then a mix: whatever was there before the
/// write, and whatever is there after.
///
/// A provider can DETECT it — `fstat` on its own descriptor before the
/// first byte and after the last, comparing size and mtime — and
/// cannot prevent it. Linux advisory locks bind only processes that
/// opt in, and mandatory locking was removed in 5.15. But there is no
/// frame here that means "here are the bytes, and they moved while I
/// sent them", so a provider that detects one has nowhere to say so.
///
/// # No length, anywhere
///
/// Not in a head, not in the request, not implied by anything. A file
/// being written to can change size in both directions after a sender
/// has looked at it, so any length stated up front is a promise made
/// about a number that has already moved. It is the commitment that
/// makes `tar` corrupt a whole archive when one entry shifts, and this
/// declines to make it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    ///
    /// Sent as they are read. A sender holds no more than one buffer's
    /// worth, so a file larger than memory crosses without either end
    /// ever holding it whole.
    pub &'a [u8],
);

/// Straight through, and identical to
/// [`write_bytes`](crate::shared::containers::write_bytes::response::Frame)
/// — which is what makes piping a read into a write cost nothing. One
/// frame out is one frame in, with no shape to translate between
/// them; whatever tag an endpoint puts in front is a byte, not a
/// copy.
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
