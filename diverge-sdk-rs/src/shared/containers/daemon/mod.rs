//! A daemon connection the container's program opened, carried to the
//! caller.
//!
//! A program in a container is a client of the daemon, and the daemon
//! lives with the caller. The program dials its proxy on the
//! loopback's `/daemon` and speaks the [daemon protocol](crate::daemon)
//! on it as a client speaks it to the daemon — scopes, channels,
//! every endpoint — and the provider carries every such connection
//! out as a PAIR of channels, one per direction, correlated by a
//! [`request::Daemon`] each carries, exactly as it carries a
//! [`postgres`](super::postgres) connection:
//!
//! - The provider opens a channel on the scope with the id it minted.
//!   What comes back on it is everything the DAEMON says: the server
//!   frames of the connection, as [`server::Frame`]s, typed; its
//!   finish is the daemon closing the connection — and an empty finish
//!   is the caller declining to connect.
//! - The caller opens a channel quoting the same id. What comes back
//!   on it is everything the PROGRAM sends: the client frames of the
//!   connection, as [`client::Frame`]s, typed; its finish is the
//!   program's socket ended.
//!
//! # Why a connection is two channels
//!
//! Because only a responder can end a channel, and a connection has
//! to be endable from both sides. A program that closes its socket
//! has to be sayable to the caller, or the daemon's session stays
//! open with nothing left to serve; a daemon that drops the
//! connection has to be sayable to the provider, or the program waits
//! on a reply that is not coming. One duplex channel could express
//! neither. Two express both with nothing added: each side finishes
//! the one it answers on, and that finish IS the close.
//!
//! # Open it, or decline
//!
//! A caller that has taken the provider's half owes it one of two
//! things: its own half, or a finish on the provider's channel. The
//! daemon protocol is client-first, so the program's first request
//! may be written the instant it connected, and the proxy holds the
//! program's frames until the caller's half arrives.
//!
//! # One connection, one session
//!
//! The daemon serves every connection for the container's
//! [`account`](crate::daemon::create::Inner::account), and no credential
//! passes: the proxy is the trust boundary, and an `Auth` frame is the
//! one frame of either side that neither [`client::Frame`] nor
//! [`server::Frame`] has — one does not decode, and the proxy closes
//! `/daemon` on one. Within a connection the program mints scopes and
//! channels as any client does, and two connections are two
//! connections: their numbers never meet, and nothing between the
//! program and the daemon reads them.
//!
//! # Several at once is the ordinary case
//!
//! A program may hold any number of daemon connections, each a pair
//! per connection, in parallel, each with its own id, its own two
//! channels and its own ordering; their frames interleave freely on
//! the socket. Nothing is parsed between the ends: a frame crosses as
//! it is, typed only so far as the wire's own header goes.
//!
//! The same shapes ride both wires this crate defines: the provider's
//! channel toward the caller, and the
//! [`proxy`](crate::container_proxy::outside::endpoints::agents::begin)
//! inside the container.

pub mod client;
pub mod request;
pub mod server;
