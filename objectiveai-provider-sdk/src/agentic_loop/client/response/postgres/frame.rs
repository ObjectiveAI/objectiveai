//! What a client's response frame carries on a Postgres channel.

use std::convert::Infallible;

use crate::encode::{Encode, Writer};

/// The payload of a
/// [`ClientFrame::Response`](crate::frame::client::ClientFrame::Response)
/// on a channel opened by
/// [`request::Frame::Postgres`](crate::agentic_loop::server::request::Frame::Postgres).
///
/// pgwire as it came off the socket, going back the way it came.
///
/// Opaque, for the reason db-proxy's conduit gives: it is never
/// parsed, so TLS negotiation and every protocol extension cross
/// untouched. And a stream rather than a message — a Postgres message
/// larger than one frame simply spans several, and both ends
/// reassemble, as they would from a socket.
///
/// # Why a struct, where MCP has an enum
///
/// Because there is nothing to choose between.
/// [`mcp::Frame`](crate::agentic_loop::client::response::mcp::Frame) has two
/// variants for a real reason: an MCP answer has a head that arrives
/// once and a body that arrives repeatedly, and a reader must tell
/// them apart. A Postgres channel has one kind of traffic from the
/// first byte to the last. An enum would imply a decision nobody
/// makes.
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
    /// mode, and saying so is better than inventing an error nobody
    /// can produce and every caller has to handle.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}
