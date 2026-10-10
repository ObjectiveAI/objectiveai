//! What a server's channel request frame carries on a connect scope.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send your frames here.
///
/// Opened once the acceptor has taken the connection: what the
/// connector answers on it, one client frame per channel response,
/// is relayed to the acceptor, and the connector finishing it is the
/// connector hanging up. It exists because only a responder streams:
/// the connector's frames have to travel as responses on a channel
/// the provider opened.
///
/// # A struct, no tag, and no payload
///
/// One thing to ask for is a struct; an enum of one variant would be
/// a discriminant with nothing to discriminate. There is nothing to
/// say beyond the opening, so the payload is empty, and bytes in it
/// are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): nothing is written.
    type Error = std::convert::Infallible;

    fn encode(&self, _: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// [`Infallible`](std::convert::Infallible): any payload is this.
    type Error = std::convert::Infallible;

    fn decode(_: &[u8]) -> Result<Self, Self::Error> {
        Ok(Frame)
    }
}
