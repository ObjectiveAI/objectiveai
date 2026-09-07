//! One chunk of the continuation being fetched.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A piece of the continuation's bytes, verbatim — no header, no
/// tag: the request asked for the one thing a run can resume from,
/// so there is nothing a frame could need to say beyond the bytes
/// themselves.
///
/// Borrowed from the frame it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver is
/// about to hand these bytes to a provider's own parser, and copying
/// them first would double every chunk's memory for nothing.
///
/// # Every frame is one chunk, replayed
///
/// A continuation is a sequence of chunks whose boundaries are part
/// of it: the client sends back the pieces the earlier run's closer
/// delivered, one frame each, in the same order — never joined,
/// never re-split — and the channel's finish says the sequence is
/// whole. Each is at most [`CHUNK_SIZE`](crate::CHUNK_SIZE),
/// because the provider that minted it kept to that.
///
/// # Zero frames is a fresh start
///
/// A finish with no frames before it is the client saying there is
/// nothing to resume — the ordinary first run, not a refusal. The
/// client is free to end the response the moment it opens.
///
/// # Whole is the provider's to judge
///
/// No identity backs these bytes, so no size or hash tells a
/// truncated delivery from a complete one here. The bytes are the
/// provider's own state in its own format; opening them is where a
/// short delivery fails, and that failure is the provider's to
/// report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The bytes, borrowed from the frame they arrived in.
    pub body: &'a [u8],
}

impl Encode for Frame<'_> {
    /// Bytes copied to bytes: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Bytes taken as bytes: nothing to fail.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Frame { body: bytes })
    }
}
