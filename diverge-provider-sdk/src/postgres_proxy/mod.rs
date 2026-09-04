//! The wire between a provider server and the Postgres proxy inside
//! a container.
//!
//! An agentic loop container runs a Postgres proxy: to the agent
//! beside it, a plain TCP listener on the container's loopback —
//! port `14980` — that its database driver dials as if it were the
//! database; to the server outside, a WebSocket listener on port
//! `14981`. Every connection the agent opens leaves the container on
//! that socket, and this module is the frames it rides. The server
//! carries each one on to the caller as the loop's channel pair —
//! [`Postgres`](crate::endpoints::agentic_loop::run::server::channel_request::Postgres)
//! toward the caller, its twin back — and the caller splices it onto
//! the real database.
//!
//! Every message is one WebSocket BINARY frame, and both directions
//! frame the same way:
//!
//! ```text
//! [type: u8][connection: u32 big-endian][payload…]
//! ```
//!
//! One byte says which, four name the connection, and the rest — on
//! the two frames that carry any — is pgwire, never parsed. Fixed
//! headers, no length prefix: WebSocket already delimits messages;
//! see [`HEADER_LEN`].
//!
//! # A connection is a socket, not an exchange
//!
//! The MCP proxy's wire is exchanges: one channel, one request, its
//! answers, a finish. This wire is sockets: a connection opens, bytes
//! flow both ways in whatever order the two ends produce them, and
//! either end closes it. Nothing is acknowledged, nothing is framed
//! as a message — a pgwire message larger than one frame spans
//! several, and each end reassembles as it would from a socket.
//!
//! # The container mints the connections
//!
//! Only the container opens connections — the agent's driver dials,
//! the proxy accepts — so there is one minter and nothing to collide
//! with. A connection is a `u32`, counted up from `1`, unique among
//! the container's LIVE connections; reuse after both ends have
//! closed is fine, because nothing remembers. It is a `u32` and not
//! the MCP proxy's `u8` for one reason: it is the SAME number the
//! server quotes as the channel pair's `connection_id` toward the
//! caller, so a relay carries it through rather than mapping it.
//!
//! # One connection, and the proxy waits for it
//!
//! The proxy accepts exactly one WebSocket connection at a time. A
//! second connection while one is live is refused. An agent's
//! connection that arrives before any WebSocket has is held — its
//! first bytes wait in the kernel's buffer — until one does.
//!
//! # A WebSocket dying kills every connection on it
//!
//! A socket cannot be resumed and pgwire cannot be replayed, so
//! there is no retry law here. When the WebSocket ends, every
//! connection riding it is dead: the proxy closes each agent-side
//! socket (the driver sees a server that hung up, and a pool
//! reconnects), and the server, on its side, finishes what it was
//! carrying for each. The next WebSocket starts with no connections.
//!
//! # Types
//!
//! | type | container → server | server → container |
//! |------|--------------------|--------------------|
//! | 0    | open               | data               |
//! | 1    | data               | close              |
//! | 2    | close              |                    |
//!
//! What each becomes on the wire to the caller is stated on the
//! variant: an open is the server's channel request, the container's
//! data and close are the responses and the finish on the caller's
//! channel, the server's data and close are the caller's responses
//! and finish on the server's.

pub mod container;
pub mod server;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
