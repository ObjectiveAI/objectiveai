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
/// channel — opened by different request types, routed to different
/// places. The variant is the routing, not the format.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerBodyFrame<'a> {
    /// On channel `0`: one chunk of the answer to the client's
    /// request.
    AgenticLoop(AgenticLoopChunk),
    /// MCP bytes, toward the client's MCP server.
    ///
    /// OPAQUE. The provider is a PROXY here — it forwards the traffic
    /// its model generates and forwards the answers back — and a proxy
    /// that parses what it carries can only lose by it: it fails on
    /// anything its schema is too old to know, it drops fields it does
    /// not model, and it re-serializes into bytes that are not the
    /// ones it was given. MCP's own spec versions by date and keeps
    /// adding extensions, so a pinned schema goes stale on someone
    /// else's release schedule.
    Mcp {
        /// The channel the server opened for this exchange.
        channel: u32,
        /// The bytes, borrowed from the frame they arrived in.
        payload: &'a [u8],
    },
    /// Postgres bytes, toward the database.
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
