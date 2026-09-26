//! One chunk of the blob being fetched.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A piece of the blob's bytes, verbatim — no header, no tag: the
/// request named ONE blob, so there is nothing a frame could need to
/// say beyond the bytes themselves.
///
/// Borrowed from the frame it arrived in: the receiver is about to
/// hand these bytes on, and copying them first would double every
/// chunk's memory for nothing.
///
/// # Every frame appends
///
/// The receiver is chunk-naive by design: each frame's bytes are
/// concatenated onto what arrived before, and the channel's finish is
/// what says the blob is whole. The SENDER splits at
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE); the receiver never measures.
/// Zero frames before the finish is the caller saying it does not
/// hold the digest at all.
///
/// # A short blob is detectable, and that is enough
///
/// A caller that dies mid-blob leaves the provider with bytes and a
/// finish it cannot tell from completion. No frame says "last one" —
/// the digest does: the provider hashes what arrived, and a partial
/// blob fails it, and is never served as the blob.
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
