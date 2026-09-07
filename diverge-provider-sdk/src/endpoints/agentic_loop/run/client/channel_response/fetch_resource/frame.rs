//! One chunk of the resource being fetched.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A piece of the resource's bytes, verbatim — no header, no field
/// name, no tag: the request named ONE resource, so there is
/// nothing a frame could need to say beyond the bytes themselves.
///
/// Borrowed from the frame it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver is
/// about to write these bytes somewhere, and copying them first
/// would double every chunk's memory for nothing.
///
/// # Every frame appends
///
/// The receiver is chunk-naive by design: each frame's bytes are
/// concatenated onto what arrived before, and the channel's finish
/// is what says the resource is whole. The SENDER splits at
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE); the receiver never
/// measures.
/// Zero frames before the finish is the client saying it does not
/// hold the identity at all.
///
/// # A short resource is detectable, and that is enough
///
/// A client that dies mid-resource leaves the server with bytes and
/// a finish it cannot tell from completion. No frame says "last
/// one" — the identity does: the size and the hash it carries are
/// exactly what a partial resource fails.
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
    /// Bytes taken as bytes: nothing to fail — an empty payload is a
    /// legitimately empty resource's one frame.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Frame { body: bytes })
    }
}
