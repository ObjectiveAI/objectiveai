//! What a server's request frame carries.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// The payload of a [`ServerFrame::ChannelRequest`](crate::frame::server::ServerFrame::ChannelRequest).
///
/// One MCP exchange, toward the client's MCP proxy. Complete in this
/// frame; the answer comes back as client response frames.
///
/// A payload leads with one byte and the rest is the request.
///
/// It is the one thing a server asks its client for, and it is a
/// connection the server cannot make itself: the agent runs beside the
/// provider, and the MCP servers live with the client. So the provider
/// opens a channel, and the client splices the far end into the real
/// thing.
///
/// # MCP is carried as exchanges, not as a socket
///
/// Because MCP over Streamable HTTP is not a connection. It is a
/// series of discrete exchanges over a session identified by a HEADER
/// rather than by anything at the transport layer.
///
/// Terminating the HTTP at each end and carrying the exchange itself
/// keeps HTTP/1.1 framing out of this protocol entirely: no chunked
/// encoding, no keep-alive boundaries, no request parser in the
/// conduit, and a terminator that can rebuild an ordinary request and
/// hand it to an ordinary router. The JSON-RPC inside stays opaque
/// regardless — see
/// [`Request::body`](crate::shared::http::request::Request::body).
///
/// # A struct, and no tag
///
/// One thing to ask for is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate — and a tag byte is that
/// discriminant written on the wire, so it goes for the same reason.
///
/// It is not held open against a second thing to ask for. A tag spent
/// on a choice nobody is making is a byte on every frame and a case in
/// every reader, paid now for something that may never happen — which
/// is the trade
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// looked at and declined.
///
/// The frame's own `type` could have carried the discrimination — it
/// is right there in the header — and deliberately does not. A frame
/// already carries one payload's worth of protocol; splitting it
/// across the envelope and the payload would mean two vocabularies to
/// version and two places to keep in step.
///
/// # Not the database
///
/// A [`plugin`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Postgres)
/// gets that channel, because a plugin is what needs a database. An
/// agent talks to its tools; a tool is what keeps something. So the
/// tunnel ends where the tool runs, and this loop never sees a
/// connection it has no query to send down.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The request, relayed verbatim.
    ///
    /// Handed off to
    /// [`Request`](crate::shared::http::request::Request)'s own impl
    /// rather than serialized here — not to save the four lines, but
    /// because an MCP request has ONE wire form, and writing it a
    /// second time in a second place is how two wire forms start.
    pub Request<'a>,
);

/// The request's own bytes, and nothing in front of them.
impl Encode for Frame<'_> {
    /// The ordinary JSON failure, which is the request's own.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        self.0.encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// The ordinary JSON failure, which is the request's own. There is
    /// nothing else here to get wrong.
    type Error = serde_json::Error;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Request::decode(bytes).map(Frame)
    }
}
