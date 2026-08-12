//! What a server's body frame carries.

use super::super::client::response::AgenticLoopChunk;

/// The payload of a [`ServerFrame::Body`](crate::frame::server::ServerFrame::Body).
///
/// A body frame's bytes mean different things on different channels,
/// and this is the set of things they can mean. Which one applies is
/// not carried in the frame: it was settled when the channel opened —
/// channel `0` by the client's request, any other by the type of the
/// server request that opened it. A reader knows before the body
/// arrives, so the body does not repeat it.
///
/// [`AgenticLoop`](Self::AgenticLoop) carries no channel because it is
/// always channel `0`. The others do, for the same reason
/// [`ServerFrame::Request`](crate::frame::server::ServerFrame::Request)
/// does: a decoded body has to stay addressable once it is no longer
/// beside the frame it came out of.
///
/// # Why two opaque variants rather than one
///
/// [`Mcp`](Self::Mcp) and [`Postgres`](Self::Postgres) are the same
/// shape, and stay separate because they are different KINDS of
/// channel — opened by different request types, spliced to different
/// places. The variant is the routing, not the format.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerBodyFrame<'a> {
    /// On channel `0`: one chunk of the answer to the client's
    /// request.
    AgenticLoop(AgenticLoopChunk),
    /// A tunneled HTTP connection, toward the client's MCP proxy.
    ///
    /// OPAQUE, and a BYTE STREAM rather than a sequence of messages:
    /// plain HTTP/1.1 exactly as it came off the socket — request
    /// line, headers, body; status line, headers, body — so a message
    /// larger than one frame simply spans several and both ends
    /// reassemble, as they would from a socket.
    ///
    /// # Why an HTTP connection and not MCP messages
    ///
    /// The agent runs in a container beside a conduit listening on
    /// loopback. It speaks ordinary Streamable HTTP to that conduit,
    /// which splices the TCP connection into this channel; the client
    /// hands the far end to its own MCP proxy, which terminates it.
    /// Neither end of this channel is an MCP implementation. Both are
    /// pipe.
    ///
    /// That has to be a connection rather than a request/response pair
    /// because MCP's answers are not all single bodies. A reply may be
    /// `text/event-stream` held open for the length of the exchange,
    /// and the server-initiated stream is a `GET` held open for the
    /// length of the SESSION. A byte stream carries those, and carries
    /// chunked encoding and large tool results, without modelling any
    /// of it.
    ///
    /// It also has to carry response HEADS, which is what rules out
    /// forwarding bare JSON-RPC. `Mcp-Session-Id` is how a client
    /// LEARNS its session id — the initialize response mints it — and
    /// `Content-Type` is what tells the client whether it is reading
    /// one object or a stream. The status line matters just as much:
    /// `202` for a bodiless notification, `404` for a session the
    /// client must re-initialize. Tunneling the connection carries all
    /// of it for free; anything narrower would have to model it.
    ///
    /// # Consequences worth knowing
    ///
    /// A channel is one TCP CONNECTION, not one exchange. Keep-alive
    /// means several request/response pairs ride the same channel, and
    /// an agent will typically hold a `GET` stream open on one channel
    /// while issuing `POST`s on another.
    ///
    /// `Origin` is not read. The MCP spec has servers validate it to
    /// defend against DNS rebinding, which is a threat to BROWSERS;
    /// there is no browser here, the tunneled value only ever names
    /// the container's own loopback address, and the frame layer is
    /// already the trust boundary. A terminator on this path that
    /// enforced it would reject every honest request.
    ///
    /// The tunnel is plaintext, and TLS inside it would buy nothing:
    /// the connection it carries never leaves the container, and this
    /// channel is authenticated already.
    Mcp {
        /// The channel the server opened for this connection.
        channel: u32,
        /// The bytes, borrowed from the frame they arrived in.
        payload: &'a [u8],
    },
    /// A tunneled Postgres connection, toward the database.
    ///
    /// OPAQUE, for the reason db-proxy's conduit gives: pgwire is
    /// never parsed, so TLS negotiation and every protocol extension
    /// cross untouched, and a Postgres message larger than one frame
    /// simply spans several. Both ends reassemble a byte stream, as
    /// they would from a socket.
    Postgres {
        /// The channel the server opened for this connection.
        channel: u32,
        /// The bytes, borrowed from the frame they arrived in.
        payload: &'a [u8],
    },
}
