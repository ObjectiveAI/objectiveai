//! One piece of a connection's traffic.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// pgwire, as one side wrote it, on its way to the other.
///
/// One type for both halves of a connection: what the database said
/// on the provider's channel, what the container wrote on the
/// caller's. Both ends of a wire carry the same current, and the
/// channel a frame arrives on already says which way it is going.
///
/// Opaque, for the reason the whole conduit is: it is never parsed,
/// so TLS negotiation and every protocol extension cross untouched.
/// And a stream rather than a message — a Postgres message larger
/// than one frame simply spans several, and both ends reassemble, as
/// they would from a socket.
///
/// # Why a struct, where the outbound side has an enum
///
/// Because there is nothing to choose between. Once a channel is open
/// its kind is settled, and it carries one kind of traffic from the
/// first byte to the last. An enum would imply a decision nobody
/// makes, and a tag byte would be a tag on a stream — a byte the far
/// end has to strip out of every write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    pub &'a [u8],
);

/// Straight through. There is no encoding step because there is
/// nothing encoded — pgwire arrives as bytes and leaves as the same
/// bytes, which is the whole of what a tunnel promises.
impl Encode for Frame<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

/// The bytes, kept. Decoding pgwire would mean parsing it, which is
/// the one thing a tunnel promises not to do.
impl<'a> Decode<'a> for Frame<'a> {
    /// [`Infallible`]: there is nothing to get wrong about a slice
    /// that is already the answer.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Frame(bytes))
    }
}
