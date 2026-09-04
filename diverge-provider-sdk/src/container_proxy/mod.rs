//! The wire between a provider server and the proxy inside a
//! container: five features, five paths, one port.
//!
//! Every container the provider runs — an AGENT container, whose
//! entrypoint runs an agentic loop on a prompt, and an MCP container,
//! whose entrypoint is an MCP connection — carries one proxy program
//! beside its own entrypoint. The proxy is how the container reaches
//! the caller's world and how the caller sees into the container,
//! and this module is the frames each of its five features rides.
//!
//! The container's own entrypoint listens on `14978`; that surface
//! belongs to the `containers` endpoint and is not described here.
//! The proxy listens on [`PORT`], `14979`, and serves five paths:
//!
//! | path        | shape    | who asks, who answers |
//! |-------------|----------|-----------------------|
//! | `/postgres` | socket   | the container's driver dials a database; the caller's answers |
//! | `/vault`    | exchange | the container reads, writes and locks keys the caller holds |
//! | `/command`  | exchange | the container runs a diverge command; the caller streams the answer |
//! | `/filetree` | stream   | the container streams its filesystem; the server reads |
//! | `/mcp`      | exchange | the container's tool calls; the caller's MCP servers answer |
//!
//! One thing does not fit on the port: the pgwire listener the
//! container's database driver dials is raw TCP, not HTTP, so it is
//! its own loopback port — [`postgres::LOOPBACK_PORT`]. And for an
//! agent container the proxy also serves the agent's MCP SERVER —
//! the Streamable HTTP endpoint its MCP client speaks to — at
//! `/mcp/agent` on this same port; that surface is MCP's own and
//! not a wire of this module.
//!
//! # The rules every path shares
//!
//! - Every message is one WebSocket BINARY frame. Text is a peer
//!   speaking something else, and the connection ends.
//! - The SERVER dials. The proxy accepts exactly one connection per
//!   path at a time; a second arrival while one is live is refused
//!   with `409` before the upgrade.
//! - There is no handshake. The upgrade at the path is it, and the
//!   first frame is protocol.
//! - The paths are independent. Each holds its own state — its live
//!   channels, its sessions, its locks — and one path's connection
//!   dying kills only that state. What a death means is each path's
//!   to say, and each says it.
//! - Nothing times anything out. A quiet connection is a connection
//!   still running.
//!
//! # Three shapes
//!
//! An EXCHANGE path ([`vault`], [`command`], [`mcp`]) is the
//! channel discipline in miniature: the container opens a channel
//! with one request, the server answers with responses on it and a
//! finish, after which the channel is dead. The container mints the
//! channels — a `u8`, unique among its live ones — so the frames are
//! `[channel: u8][request…]` outward and `[type: u8][channel:
//! u8][payload…]` back, the type saying response or finish. Whether
//! a channel that died with its connection is asked again is the
//! path's rule: [`mcp`] re-asks, [`vault`] and [`command`] do not,
//! and each says why.
//!
//! A SOCKET path ([`postgres`]) carries connections, not exchanges:
//! bytes both ways in whatever order the two ends produce them,
//! several connections at once, each named by a `u32` the container
//! minted, `[type: u8][connection: u32][payload…]` in both
//! directions.
//!
//! A STREAM path ([`filetree`]) is one direction only: the container
//! sends frames for as long as the connection lives, and the server
//! sends nothing.

pub mod command;
pub mod filetree;
pub mod mcp;
pub mod postgres;
pub mod vault;

/// The port the proxy listens on, inside the container, serving the
/// five paths. The server dials it.
pub const PORT: u16 = 14979;
