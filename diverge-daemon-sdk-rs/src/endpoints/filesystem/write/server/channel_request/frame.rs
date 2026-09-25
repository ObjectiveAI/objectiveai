//! What a server's channel request frame carries for a filesystem
//! write.

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};

/// Send the content.
///
/// What the daemon asks the client for, and the only thing: a write
/// cannot carry its own content, because only a responder can finish
/// a channel, so the bytes have to travel as responses on a channel
/// the daemon opened. Opened once the destination is checked, and
/// before anything is written.
///
/// # A struct, no tag, and no id
///
/// One thing to ask for is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate — and a tag byte is that
/// discriminant written on the wire, so it goes for the same reason.
/// A container's
/// [`write_bytes`](diverge_provider_sdk::shared::containers::write_bytes)
/// quotes a write id, because there the content channel and the write
/// share a scope with every other write; here the scope IS the write,
/// so a payload is nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// Nothing at all. The frame is the whole message: which scope it
/// arrived on says which write.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): no bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// [`Infallible`](std::convert::Infallible): there is nothing to
    /// read.
    type Error = std::convert::Infallible;

    /// Whatever bytes arrive are ignored. There are none to send, so a
    /// peer that sent some knows something this version does not, and
    /// leaving room for it is cheaper than refusing it.
    fn decode(_bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Frame)
    }
}
