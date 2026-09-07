//! What a server's response frame carries for a tool container connection.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A connection's answer, which is a failure or nothing.
///
/// One struct, no tag, at most one frame, usually zero:
///
/// | the scope | means |
/// |-----------|-------|
/// | says nothing, and stays open | the connector is attached |
/// | one of these, then a finish | it never was — the container was not there, its runner said no |
/// | a finish, with none of these | the connection is over — the connector left, or the container did |
///
/// A connector already holds the id, so there is nothing to tell it;
/// everything it reads from the container is a channel it opens.
/// An enum of one variant would be a discriminant with nothing to
/// discriminate, and a tag byte is that discriminant written on the
/// wire, so it goes for the same reason.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// Why the connection is not open. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    pub Error,
);

impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        self.0.encode(out)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        Error::decode(bytes).map(Frame)
    }
}
