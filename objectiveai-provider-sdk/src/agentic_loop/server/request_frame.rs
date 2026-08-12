//! What a server's request frame carries.

/// The payload of a [`ServerFrame::Request`](crate::frame::server::ServerFrame::Request).
///
/// A server asks its client for two things, and both are the same ask
/// in different clothes: a connection it cannot make itself. The agent
/// runs beside the provider; the MCP servers and the database live
/// with the client. So the provider opens a channel and writes down
/// it, and the client splices the far end into the real thing.
///
/// Which variant applies is the frame's own type, not anything in the
/// payload — the server's type space is open above `4` for exactly
/// this. Once a channel is open its kind is settled, and the bytes
/// that come back need no tag at all.
///
/// # Both are opaque byte streams
///
/// Neither is parsed, and neither is a sequence of messages. These are
/// SOCKETS: what the agent wrote, verbatim, in the order it wrote it.
/// A message larger than one frame simply spans several, and
/// successive request frames on one channel are successive writes — a
/// channel is a CONNECTION, not an exchange.
///
/// That is what keeps both tunnels honest. db-proxy's conduit never
/// parses pgwire, so TLS negotiation and every protocol extension
/// cross untouched; the same reasoning applies to MCP, whose spec
/// versions by date and keeps adding extensions. A relay that parsed
/// what it carried could only fail on what its schema was too old to
/// know, drop fields it did not model, and hand on bytes that were not
/// the ones it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerRequestFrame<'a> {
    /// A tunneled HTTP connection, toward the client's MCP proxy.
    ///
    /// Plain HTTP/1.1 as it came off the socket. The agent speaks
    /// ordinary Streamable HTTP to a conduit on loopback; the conduit
    /// splices that connection into this channel; the client hands the
    /// far end to its own MCP proxy, which terminates it. Neither end
    /// of the channel is an MCP implementation — both are pipe.
    ///
    /// It is a CONNECTION rather than a request and a reply because
    /// MCP's answers are not all single bodies: a reply may be
    /// `text/event-stream` held open for the exchange, and the
    /// server-initiated stream is a `GET` held open for the SESSION.
    /// A byte stream carries those, plus chunked encoding and large
    /// tool results, without modelling any of it.
    ///
    /// It also carries response HEADS, which is what rules out
    /// forwarding bare JSON-RPC. `Mcp-Session-Id` is how a client
    /// LEARNS its session id — the initialize response mints it —
    /// and `Content-Type` is what says whether the client is reading
    /// one object or a stream. The status line matters as much: `202`
    /// for a bodiless notification, `404` for a session that must be
    /// re-initialized.
    ///
    /// `Origin` is NOT read at the far end. The MCP spec has servers
    /// validate it against DNS rebinding, a threat to BROWSERS; there
    /// is none here, the tunneled value only ever names the agent's
    /// own loopback address, and this protocol is already the trust
    /// boundary. A terminator that enforced it would reject every
    /// honest request.
    ///
    /// The tunnel is plaintext. TLS inside it would buy nothing: the
    /// connection it carries never leaves the machine it started on,
    /// and the channel is authenticated already.
    Mcp(&'a [u8]),
    /// A tunneled Postgres connection, toward the database.
    ///
    /// pgwire as it came off the socket, for the reason db-proxy's
    /// conduit gives: it is never parsed, so TLS negotiation and every
    /// protocol extension cross untouched. Both ends reassemble a byte
    /// stream, as they would from a socket.
    Postgres(&'a [u8]),
}
