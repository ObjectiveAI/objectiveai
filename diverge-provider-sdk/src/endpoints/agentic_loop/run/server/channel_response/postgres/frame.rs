//! What a server's response frame carries on a Postgres channel.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a
/// [`ServerFrame::ChannelResponse`](crate::frame::server::ServerFrame::ChannelResponse)
/// on a channel opened by
/// [`client::channel_request::Postgres`](crate::endpoints::agentic_loop::run::client::channel_request::Postgres).
///
/// pgwire as the container wrote it, on its way to the caller's
/// database. The same bytes the
/// [plugin endpoint's](crate::endpoints::mcp_plugin::run::server::channel_response::postgres::Frame)
/// carries, for the same reason.
///
/// Opaque: it is never parsed, so TLS negotiation and every protocol
/// extension cross untouched. And a stream rather than a message — a
/// Postgres message larger than one frame simply spans several, and
/// both ends reassemble, as they would from a socket.
///
/// # The mirror of the caller's, and deliberately its own type
///
/// The [`client's`](crate::endpoints::agentic_loop::run::client::channel_response::postgres::Frame)
/// is the same bytes going the other way, and the two are written out
/// separately rather than shared. Each names one direction of one
/// connection, and what they carry is the same only in the sense that
/// both ends of a wire carry the same current.
///
/// # What the finish means here
///
/// The container's socket ended, and no further byte will ever be
/// written by this connection. That is the signal that has no other
/// way to travel: a process that crashes sends no pgwire `Terminate`,
/// so without it a caller would hold a backend for a client that no
/// longer exists. It is the whole reason this channel is the caller's
/// to open — a provider cannot finish a channel it is asking on, so
/// the writes had to become an answer for the provider to be able to
/// end them.
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
