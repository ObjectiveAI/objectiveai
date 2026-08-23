//! What a registry answer carries back.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A piece of the registry's answer.
///
/// Bytes, and nothing in front of them. A status line, headers and a
/// body are all the same thing here — bytes off a socket — and where
/// one ends is HTTP's business rather than this channel's.
///
/// # There is no head
///
/// There was, carrying a status and a header map. It went with the
/// parsing: a relay that hands over a status has read one, and a relay
/// that has read one must then decide what to do about the headers that
/// describe the MESSAGE rather than the answer. Every such decision was
/// a bug — a `Content-Length` copied onto a body that had been
/// re-encoded, a `Connection` forwarded past the hop it belonged to.
///
/// # And no error
///
/// Which is the part worth explaining, because most channels in this
/// crate have one.
///
/// A registry that refuses already knows how to say so: a `404`, a
/// `401`, a `429` with a `Retry-After`, an error document with a code
/// in it. All of that is the answer, and travels as bytes like any
/// other answer.
///
/// And a caller that cannot reach its registry at all is a PROXY that
/// cannot reach its upstream, which HTTP also has a way to say. It
/// answers `502` and the runtime does what it would do with any `502`.
/// An error variant beside that would be a second vocabulary for
/// something the first one already covers, and a runtime would have to
/// understand both to learn one thing.
///
/// So the pieces mean nothing, are not required to align with anything,
/// and never say anything about themselves. A caller sends what it has
/// when it has it; a provider writes it onto the runtime's socket in
/// the order it arrives. That is what makes a layer of hundreds of
/// megabytes possible without either end holding one.
///
/// # The channel's finish is the end
///
/// Nothing in the payload says the answer is over, because a finish
/// already does and a second signal for one fact is a second thing to
/// disagree about. A runtime that has been told a `Content-Length`
/// knows when it has the whole body; one reading a chunked answer knows
/// from the terminator. Both of those are inside the bytes, where they
/// belong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    pub &'a [u8],
);

/// Straight through. There is no encoding step because there is
/// nothing encoded — an answer arrives as bytes and leaves as the same
/// bytes.
impl Encode for Frame<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode, and saying so is better than inventing an error nobody can
    /// produce and every caller has to handle.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// [`Infallible`]: every byte string is one of these, including the
    /// empty one — which is a piece that happened to carry nothing and
    /// is ordinary on a socket.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Frame(bytes))
    }
}
