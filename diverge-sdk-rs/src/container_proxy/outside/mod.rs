//! What a provider's server asks of the proxy inside a container,
//! over one connection.
//!
//! Every container the provider runs carries one proxy program beside
//! its own entrypoint, listening for the server on [`OUTSIDE_PORT`].
//! The server dials it ONCE, and everything the two say to each other for the
//! container's life rides that one WebSocket, framed exactly as the
//! provider protocol frames its own wire: [`frame`](crate::wire::frame),
//! nine bytes of header — a type, a scope, a channel — and a payload.
//! Same header, same seven types, same rules; different requests.
//!
//! # The server is the client here
//!
//! On this wire the provider's SERVER dials, mints scopes, and sends
//! [`ClientFrame`](crate::wire::frame::client::ClientFrame)s; the proxy
//! answers with [`ServerFrame`](crate::wire::frame::server::ServerFrame)s.
//! Under every scope below, `client` is therefore the server's
//! traffic and `server` the proxy's — which reads oddly once, and then
//! reads as the frame layer does: the client is whoever opened the
//! connection and the scopes, whatever it is called elsewhere.
//!
//! # No authorization
//!
//! Type `0` is never sent. The server reached the proxy by a route
//! only it has — the port is published to the provider and to no one
//! else — and there is nothing for either end to prove. A peer that
//! sends an auth frame is speaking some other protocol, and the
//! connection is closed on it.
//!
//! # What is here
//!
//! [`endpoints`] is the request vocabulary: the six scopes the server
//! opens on the proxy, each with what it sends and what comes back,
//! and the tag table that names them. [`client`] is what the six
//! scopes' executors share — the asks a proxy opens on a scope, read
//! as a stream — for the provider's server, which is the client
//! here.
//!
//! # And what stays
//!
//! The program inside the container still has the proxy's own
//! listeners on the loopback — its HTTP surface, and the Postgres
//! port its driver dials — which are the program's surfaces and not
//! wires of this module; what arrives on them becomes a channel
//! request on `begin`.
//!
//! # And a way to speak it
//!
//! Every scope's `client::execute` performs the exchange rather than
//! describing it, for the provider's server: hand it the
//! [`Handle`](crate::wire::client::handle::Handle) that
//! [`proxy::dial`](crate::provider::server::proxy::dial) made and what the
//! request carries, and get back the scope's answer — the begin's
//! asks and chunks, a mount's asks, a tree's frames, a file's bytes,
//! a write's fate. The party that executes here is the provider; what
//! it uses is the frame-level [`client`](crate::wire::client), the
//! same one a caller of the provider holds.

pub mod client;
pub mod endpoints;

/// The port the proxy listens for the server on, inside the
/// container: the one WebSocket of this module is dialled here, from
/// outside, at the root path.
///
/// One port, always the same one, so a
/// [`Deployment`](crate::provider::server::deployment::Deployment) names none:
/// a deployer makes it reachable on every container it deploys and
/// reports where, and the entrypoint's own port is behind the proxy,
/// on the loopback inside, never published.
pub const OUTSIDE_PORT: u16 = 14979;
