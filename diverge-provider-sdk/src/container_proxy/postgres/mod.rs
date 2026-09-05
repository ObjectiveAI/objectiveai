//! Postgres: the container's database connections, carried to the
//! caller as raw conduits.
//!
//! The proxy is a database to the container: a plain TCP listener on
//! the container's loopback, [`LOOPBACK_PORT`], which the container's
//! driver dials as if it were Postgres. Each connection the driver
//! opens is ANNOUNCED on `/requests` — a
//! [`Postgres`](crate::container_proxy::requests::Request::Postgres)
//! ask, kind `11`, carrying nothing but its channel — and the server
//! opens `/postgres/{channel}` for it. That WebSocket is the
//! connection: raw pgwire in both directions, one message one chunk,
//! never parsed, no frame around it, until either side closes.
//!
//! ```text
//! the ask, after the channel:   [11]
//! the conduit, either way:      [pgwire bytes…]
//! ```
//!
//! # A conduit, not a protocol
//!
//! Nothing is acknowledged, nothing is framed as a message — a
//! pgwire message larger than one WebSocket message spans several,
//! and each end reassembles as it would from a socket. The bytes are
//! never parsed, which is what lets TLS negotiation and every
//! protocol extension cross untouched. There is no open frame and no
//! close frame: the path opening is the connection existing, and the
//! WebSocket closing — cleanly or not — is the connection ending, on
//! whichever side closed it.
//!
//! # The driver may dial before the path exists
//!
//! pgwire is client-first: the driver writes its startup message the
//! instant it connects. The proxy holds the socket — its first bytes
//! wait in the kernel's buffer — until the server has opened the
//! path, then pumps. A driver that dials before any `/requests`
//! connection exists is held until one does and the announcement can
//! be sent.
//!
//! # What dying means here
//!
//! The path ending is the connection ending: the proxy shuts the
//! driver's socket, the driver sees a server that hung up, and a
//! pool reconnects — which is a new announcement and a new path.
//! `/requests` dying with an announcement not yet opened is the same
//! from the driver's side: its socket is shut. A socket cannot be
//! resumed and pgwire cannot be replayed, so nothing is retried.
//!
//! No types: there is no frame here, only bytes.

/// The port the container's database driver dials, on the
/// container's loopback: the proxy's pgwire listener. Not on the
/// proxy's own [`PORT`](super::PORT), because pgwire is not HTTP and
/// cannot share a listener with the paths.
pub const LOOPBACK_PORT: u16 = 14980;
