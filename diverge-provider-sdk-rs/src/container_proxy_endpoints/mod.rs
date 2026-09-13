//! What a provider's server asks of the proxy inside a container,
//! over one connection.
//!
//! Every container the provider runs carries one proxy program beside
//! its own entrypoint, listening for the server on
//! [`OUTSIDE_PORT`](crate::container_proxy::OUTSIDE_PORT). The server
//! dials it ONCE, and everything the two say to each other for the
//! container's life rides that one WebSocket, framed exactly as the
//! provider protocol frames its own wire: [`frame`](crate::frame),
//! nine bytes of header — a type, a scope, a channel — and a payload.
//! Same header, same seven types, same rules; different requests.
//!
//! # The server is the client here
//!
//! On this wire the provider's SERVER dials, mints scopes, and sends
//! [`ClientFrame`](crate::frame::client::ClientFrame)s; the proxy
//! answers with [`ServerFrame`](crate::frame::server::ServerFrame)s.
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
//! # The scopes
//!
//! Each of these is one SCOPE — a request that opens one, whatever
//! channels either side needs inside it, and the answer that comes
//! back on channel `0`.
//!
//! | tag | request |
//! |-----|---------|
//! | `0` | [`agents::begin`] |
//! | `1` | [`tools::begin`] |
//! | `2` | [`fuse::mount`] |
//! | `3` | [`filesystem::tree`] |
//! | `4` | [`filesystem::read`] |
//! | `5` | [`filesystem::write`] |
//!
//! Six. A `begin` leads, one per family, because it is the server's
//! first act on every connection: the scope the proxy's own asks ride
//! — the container's database connections, its commands, its vault,
//! its tool calls outward — and the family's own exchanges with it. A
//! mount is a scope because the asks a mount makes ride it, and a
//! tree is one because it does not end by itself. A read and a write
//! are scopes because each is a stream of its own.
//!
//! This table is the whole allocation. Each request states its own
//! value and points here, because a value chosen in one module has to
//! be checked against every other, and no module can see the others.
//!
//! [`ClientRequest`] is the same table as a type: one variant per row,
//! in tag order, plus an [`Invalid`](ClientRequest::Invalid) for a
//! payload that is none of them. It is the only place the values meet.
//!
//! # The order the server opens them in
//!
//! `begin` first, before anything else, and exactly once. Then every
//! FUSE mount, each its own scope, and every one answered before the
//! next step. Then, on an agent container, the agent is registered — a
//! channel on `begin`. Then trees, reads, writes and the family's own
//! exchanges, as the server pleases, in parallel and in any order.
//! A channel on `begin` is opened, by either side, only after its
//! [`Begun`](agents::begin::server::response::Frame::Begun) has
//! arrived.
//!
//! # What ends
//!
//! Nothing but a finish ends a channel or a scope, however long it
//! has been quiet — the rule the frame layer states, and it holds
//! here. The connection ending is the server gone: every scope on it
//! is over, every channel with them, and nothing is re-asked. There
//! is no second connection to a proxy. The container's life is one
//! connection, and what was in flight when it ended is what was in
//! flight when the container ended.
//!
//! # And what stays
//!
//! The program inside the container still has the proxy's own
//! listeners — [`INSIDE_PORT`](crate::container_proxy::INSIDE_PORT)
//! and the Postgres loopback — which are the program's surfaces and
//! not wires of this module; what arrives on them becomes a channel
//! request on `begin`.

mod client_request;

pub use client_request::*;

pub mod agents;
pub mod filesystem;
pub mod fuse;
pub mod tools;
