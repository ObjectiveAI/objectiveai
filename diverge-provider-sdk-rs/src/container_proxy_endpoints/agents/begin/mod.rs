//! Beginning the server's work on an agent container.
//!
//! Split by who SENDS: [`client`] is the server's traffic, [`server`]
//! the proxy's.
//!
//! The scope a begin opens is the CONNECTION's life. It is the
//! server's first act, once, and it carries nothing: what it opens is
//! the place channels go. The proxy answers
//! [`Begun`](server::response::Frame::Begun) on channel `0` and then
//! nothing more there for as long as the connection lives; the finish
//! is the proxy ending.
//!
//! # Channels go both ways here
//!
//! The proxy opens them for everything the container asks of the
//! world outside — its database connections, its commands, its vault,
//! its tool calls outward. The server opens them for the family's own
//! exchanges — the agent's registration, its loops, its schema, its
//! queue —
//! and for its half of each database connection. Same scope, opposite
//! directions, and neither side's channel numbers mean anything to the
//! other.

pub mod client;
pub mod server;
