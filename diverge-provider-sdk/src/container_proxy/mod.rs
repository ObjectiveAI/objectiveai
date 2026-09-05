//! The wire between a provider server and the proxy inside a
//! container: one path for every request, a path per answer, and
//! one stream.
//!
//! Every container the provider runs — an AGENT container, whose
//! entrypoint runs an agentic loop on a prompt, and an MCP container,
//! whose entrypoint is an MCP connection — carries one proxy program
//! beside its own entrypoint. The proxy is how the container reaches
//! the caller's world and how the caller sees into the container,
//! and this module is what rides its WebSockets.
//!
//! The container's own entrypoint listens on `14978`; that surface
//! belongs to the `containers` endpoint and is not described here.
//! The proxy listens on [`PORT`], `14979`, and the server dials
//! every path on it:
//!
//! | path                                | carries |
//! |-------------------------------------|---------|
//! | `/requests`                         | every request the container makes; nothing comes back on it |
//! | `/mcp/list-tools/{channel}` and its three siblings | one MCP response, then the close |
//! | `/mcp/notifications/{channel}`      | notifications as they come, then the close |
//! | `/vault/get/{channel}` and its four siblings | one vault answer, then the close |
//! | `/command/{channel}`                | the command's items, then the close |
//! | `/postgres/{channel}`               | raw pgwire, both ways, until either side closes |
//! | `/filetree`                         | filetree frames, sent by the container; the server is silent |
//!
//! One thing does not fit on the port: the pgwire listener the
//! container's database driver dials is raw TCP, not HTTP, so it is
//! its own loopback port — [`postgres::LOOPBACK_PORT`]. And for an
//! agent container the proxy also serves the agent's MCP SERVER —
//! the Streamable HTTP endpoint its MCP client speaks to — at
//! `/mcp/agent` on this same port; that surface is MCP's own and
//! not a wire of this module.
//!
//! # A request is a frame; an answer is a WebSocket
//!
//! The container asks on [`/requests`](requests): one
//! [`Frame`](requests::Frame) per ask, carrying a CHANNEL the
//! container minted and the ask itself. The server answers by
//! opening a WebSocket at the ask's own path with that channel in
//! it, sending the answer as RAW messages — the response type and
//! nothing around it: no channel, no type byte, no finish — and then
//! CLOSING. The close is the end of the answer, which is why nothing
//! in a response message has to say so. An answer path opened and
//! closed cleanly with no message is the server saying the request
//! could not be served.
//!
//! The channel is a `u32` the container counts up, unique among its
//! requests not yet answered. A u32 rather than a small tag,
//! because a channel now names a WebSocket rather than a slot in a
//! table, and a database connection is one of them.
//!
//! # The rules every path shares
//!
//! - Every message is one WebSocket BINARY frame. Text is a peer
//!   speaking something else, and the connection ends.
//! - The SERVER dials. `/requests` and `/filetree` accept exactly
//!   one connection at a time, a second refused with `409` before
//!   the upgrade. An answer path
//!   is accepted for a channel the container announced and the
//!   server has not yet opened — an unknown channel is refused with
//!   `404`, a second opening with `409`.
//! - There is no handshake. The upgrade is it.
//! - A clean close — the Close frame, the handshake — is an answer
//!   complete or a session ended; an abrupt end is a connection
//!   dying, and the two are told apart.
//! - Nothing times anything out. A quiet connection is a connection
//!   still running.
//!
//! # What dying means, stated once
//!
//! `/requests` dying loses the requests not yet answered. What
//! happens to each is its kind's rule: the [`mcp`] exchanges are
//! asked again on the next connection, at-least-once accepted,
//! because the agent inside is waiting; a [`vault`] operation and a
//! [`command`] are reported to whoever asked as failed, because
//! neither is safe to repeat (a lock already held outlives the dead
//! connection by its TTL); a [`postgres`] announcement whose path
//! was never opened is a driver socket the proxy closes. An answer
//! path dying is that one request failing, by the same rule per
//! kind — and a postgres path dying is that session ending, the
//! driver's socket shut. The stream path ([`filetree`]) simply
//! starts over on the next connection.

pub mod command;
pub mod filetree;
pub mod mcp;
pub mod postgres;
pub mod requests;
pub mod vault;

/// The port the proxy listens on, inside the container. The server
/// dials every path on it.
pub const PORT: u16 = 14979;
