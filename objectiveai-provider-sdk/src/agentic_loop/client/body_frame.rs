//! What a client's body frame carries.

/// The payload of a [`ClientFrame::Body`](crate::frame::client::ClientFrame::Body).
///
/// The return half of each tunnel: what came back up the socket the
/// client spliced onto the far end of one of the server's channels.
/// A client never streams anything of its own — its own request rides
/// in a request frame, and channel `0` belongs to the server — so
/// every body frame it sends is an answer on a channel the SERVER
/// opened.
///
/// Nothing on the wire distinguishes these. A client body frame has no
/// type byte, and needs none: the channel's kind was settled by the
/// [`ServerRequestFrame`](super::super::server::ServerRequestFrame)
/// that opened it, and both ends have known it ever since. The variant
/// is recovered from that, not from the bytes.
///
/// Both are opaque byte streams, for the reasons given on
/// [`ServerRequestFrame`](super::super::server::ServerRequestFrame) —
/// these are SOCKETS, and a message larger than one frame simply spans
/// several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientBodyFrame<'a> {
    /// HTTP response bytes, from the client's MCP proxy.
    Mcp(&'a [u8]),
    /// pgwire bytes, from the database.
    Postgres(&'a [u8]),
}
