//! What a server's channel request frame carries for an agent container
//! connection.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::write_bytes;

/// Send the content for a write.
///
/// What a provider asks a connector for, and never unprompted. A
/// provider wants nothing from a connector on its own account — the
/// image was somebody else's problem, so is deciding who may attach,
/// and so are the container's own asks, which go to its runner. This
/// exists only because a write cannot carry its own content: only a
/// responder can finish a channel, so the bytes have to travel as
/// responses on a channel the provider opened.
///
/// # A struct, and no tag
///
/// One thing to ask for is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate — and a tag byte is that
/// discriminant written on the wire, so it goes for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame(
    /// Which write, by the id the connector gave it.
    pub write_bytes::request::Request,
);

/// The write id's four bytes, and nothing in front of them.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): four known bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        self.0.encode(out)
    }
}

impl Decode<'_> for Frame {
    /// One way to fail: a payload that was not four bytes.
    type Error = write_bytes::request::RequestError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        write_bytes::request::Request::decode(bytes).map(Frame)
    }
}
