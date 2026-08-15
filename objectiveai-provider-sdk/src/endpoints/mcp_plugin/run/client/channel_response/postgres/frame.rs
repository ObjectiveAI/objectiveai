//! What a client's response frame carries on a Postgres channel.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a
/// [`ClientFrame::ChannelResponse`](crate::frame::client::ClientFrame::ChannelResponse)
/// on a channel opened by
/// [`channel_request::Frame::Postgres`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres).
///
/// pgwire as it came off the socket, going back the way it came.
///
/// Opaque, for the reason db-proxy's conduit gives: it is never
/// parsed, so TLS negotiation and every protocol extension cross
/// untouched. And a stream rather than a message — a Postgres message
/// larger than one frame simply spans several, and both ends
/// reassemble, as they would from a socket.
///
/// # Why a struct, where the outbound side has an enum
///
/// Because there is nothing to choose between. A provider's
/// [`channel_request::Frame`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame)
/// has two variants because it opens two kinds of channel and a reader
/// must tell them apart. Once THIS channel is open its kind is
/// settled, and it carries one kind of traffic from the first byte to
/// the last. An enum would imply a decision nobody makes, and a tag
/// byte would be a tag on a stream — a byte the far end has to strip
/// out of every write.
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
