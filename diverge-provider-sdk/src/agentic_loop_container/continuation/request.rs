//! One chunk of the continuation being delivered.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The body of one `POST /continuation` — a piece of the
/// continuation's bytes, verbatim. No tag, no framing, no JSON: the
/// route already says this is a chunk of the one thing a run can
/// resume from, so there is nothing left for the body to say beyond
/// the bytes themselves — which also means no delivery can be
/// malformed.
///
/// Borrowed from the request it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver
/// is about to store these bytes as one chunk, and copying them
/// first would double every chunk's memory for nothing.
///
/// # Every POST is one chunk, kept as such
///
/// The chunk the earlier run's closer sent, replayed with its
/// boundaries intact — the store keeps the sequence, never joins
/// it. Each is at most
/// [`CHUNK_SIZE`](crate::CHUNK_SIZE),
/// the minting container's rule. A chunk may be as short as its
/// container made it (one tag byte, say); a fresh start is a lone
/// [`complete`](super::complete) with no chunks at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Request<'a> {
    /// This chunk of the bytes, borrowed from the request they
    /// arrived in.
    pub body: &'a [u8],
}

impl Encode for Request<'_> {
    /// Bytes copied to bytes: nothing to fail.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// Bytes taken as bytes: nothing to fail.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Request { body: bytes })
    }
}
