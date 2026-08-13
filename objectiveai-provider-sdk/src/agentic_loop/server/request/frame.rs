//! What a server's request frame carries.

use crate::mcp::request::Request;

/// The payload of a [`ServerFrame::Request`](crate::frame::server::ServerFrame::Request).
///
/// A server asks its client for two things, and both are the same ask
/// in different clothes: a connection it cannot make itself. The agent
/// runs beside the provider; the MCP servers and the database live
/// with the client. So the provider opens a channel, and the client
/// splices the far end into the real thing.
///
/// Which variant applies is the frame's own type, not anything in the
/// payload — the server's type space is open above `4` for exactly
/// this. Once a channel is open its kind is settled, and what comes
/// back on it needs no tag at all.
///
/// # Why the two are shaped differently
///
/// [`Postgres`](Self::Postgres) is a byte stream and
/// [`Mcp`](Self::Mcp) is a structured exchange, because pgwire really
/// is a CONNECTION and MCP over Streamable HTTP really is not.
///
/// A Postgres session is a long-lived socket carrying a conversation
/// with no natural top-level unit, so successive request frames on one
/// channel are successive writes, and a message larger than one frame
/// simply spans several. Never parsing it is what lets TLS negotiation
/// and every protocol extension cross untouched — the argument
/// db-proxy's conduit already makes.
///
/// MCP is a series of discrete exchanges over a session identified by
/// a HEADER rather than by any connection. Terminating the HTTP at
/// each end and carrying the exchange itself keeps HTTP/1.1 framing
/// out of this protocol entirely: no chunked encoding, no keep-alive
/// boundaries, no request parser in the conduit, and a terminator that
/// can rebuild an ordinary request and hand it to an ordinary router.
/// The JSON-RPC inside stays opaque regardless — see
/// [`mcp::request::Request::body`](crate::mcp::request::Request::body).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the client's MCP proxy. Complete in
    /// this frame; the answer comes back as client response frames.
    Mcp(Request<'a>),
    /// Postgres bytes, toward the database. Opaque, and a stream —
    /// this is a socket, and successive frames on the channel are
    /// successive writes.
    Postgres(&'a [u8]),
}
