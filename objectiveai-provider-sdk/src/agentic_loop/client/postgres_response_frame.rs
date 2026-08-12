//! What a client's response frame carries on a Postgres channel.

/// The payload of a
/// [`ClientFrame::Response`](crate::frame::client::ClientFrame::Response)
/// on a channel opened by
/// [`ServerRequestFrame::Postgres`](super::super::server::ServerRequestFrame::Postgres).
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
/// [`McpResponseFrame`](super::McpResponseFrame) has two
/// variants for a real reason: an MCP answer has a head that arrives
/// once and a body that arrives repeatedly, and a reader must tell
/// them apart. A Postgres channel has one kind of traffic from the
/// first byte to the last. An enum would imply a decision nobody
/// makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostgresResponseFrame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    pub &'a [u8],
);
