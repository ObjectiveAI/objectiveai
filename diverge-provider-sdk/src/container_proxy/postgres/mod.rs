//! The `/postgres` path: the container's database connections,
//! carried to the caller.
//!
//! The proxy is a database to the container: a plain TCP listener on
//! the container's loopback, [`LOOPBACK_PORT`], which the container's
//! driver dials as if it were Postgres. Every connection it opens is
//! carried out over this path, and the server carries each one on to
//! the caller, whose real database answers.
//!
//! Both directions frame the same way:
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
//! Bytes flow both ways in whatever order the two ends produce them,
//! and either end closes. Nothing is acknowledged, nothing is framed
//! as a message — a pgwire message larger than one frame spans
//! several, and each end reassembles as it would from a socket. The
//! bytes are never parsed, which is what lets TLS negotiation and
//! every protocol extension cross untouched.
//!
//! # The container mints the connections
//!
//! Only the container opens connections — its driver dials, the
//! proxy accepts — so there is one minter and nothing to collide
//! with. A connection is a `u32`, counted up from `1`, unique among
//! the container's LIVE connections; reuse after both ends have
//! closed is fine, because nothing remembers.
//!
//! # A connection that arrives before the server is held
//!
//! The driver may dial before the server has connected to this path.
//! The proxy holds the socket — its first bytes wait in the kernel's
//! buffer — until a server connection exists, then announces it.
//!
//! # The path dying kills every connection on it
//!
//! A socket cannot be resumed and pgwire cannot be replayed, so there
//! is no retry law here. When the server's connection to this path
//! ends, every database connection riding it is dead: the proxy
//! closes each driver-side socket (the driver sees a server that
//! hung up, and a pool reconnects), and the server finishes what it
//! was carrying for each. The next connection starts with none.
//!
//! # Types
//!
//! | type | container → server | server → container |
//! |------|--------------------|--------------------|
//! | 0    | open               | data               |
//! | 1    | data               | close              |
//! | 2    | close              |                    |

pub mod container;
pub mod server;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;

/// The port the container's database driver dials, on the
/// container's loopback: the proxy's pgwire listener. Not on the
/// proxy's own [`PORT`](super::PORT), because pgwire is not HTTP and
/// cannot share a listener with the paths.
pub const LOOPBACK_PORT: u16 = 14980;
