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
/// is about to append these bytes somewhere, and copying them first
/// would double every chunk's memory for nothing.
///
/// # Every chunk appends
///
/// Onto what arrived before — chunk-naive by design: next POST,
/// append. The sender splits at
/// [`CHUNK_SIZE`](crate::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE)
/// and only splits what exceeds it, so an empty chunk does not
/// occur; a fresh start is a lone [`complete`](super::complete).
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
