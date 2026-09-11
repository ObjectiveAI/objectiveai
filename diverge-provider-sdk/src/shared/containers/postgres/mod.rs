//! A database connection the container opened, carried to the caller.
//!
//! Something inside the container dials Postgres, and the database
//! lives with the caller. The provider carries every such connection
//! out as a PAIR of channels, one per direction, correlated by a
//! [`request::Postgres`] each carries:
//!
//! - The provider opens a channel on the scope with the id it minted.
//!   What comes back on it is everything the DATABASE says, as
//!   [`response::Frame`]s; its finish is the database connection
//!   closed — and an empty finish is the caller declining to dial.
//! - The caller opens a channel quoting the same id. What comes back
//!   on it is everything the CONTAINER wrote, as [`response::Frame`]s;
//!   its finish is the container's socket ended.
//!
//! # Why a connection is two channels
//!
//! Because only a responder can end a channel, and a connection has
//! to be endable from both sides. A container that dies has to be
//! sayable to the caller, or the caller's backend stays open with
//! nothing left to serve; a database that drops has to be sayable to
//! the provider, or the container waits on a reply that is not
//! coming. One duplex channel could express neither. Two express both
//! with nothing added: each side finishes the one it answers on, and
//! that finish IS the close.
//!
//! # Open it, or decline
//!
//! A caller that has taken the provider's half owes it one of two
//! things: its own half, or a finish on the provider's channel. pgwire
//! is client-first, so the container wrote its startup message the
//! instant it connected and the provider is holding bytes until the
//! caller's half arrives.
//!
//! # Several at once is the ordinary case
//!
//! A pool is a pair per connection, in parallel, each with its own id,
//! its own two channels and its own ordering; their frames interleave
//! freely on the socket. Nothing is parsed: TLS negotiation and every
//! protocol extension cross untouched, and a message larger than one
//! frame spans several, both ends reassembling as from a socket.

pub mod request;
pub mod response;
