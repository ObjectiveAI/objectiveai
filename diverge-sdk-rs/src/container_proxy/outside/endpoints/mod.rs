//! What a provider's server asks of the proxy inside a container:
//! the six scopes, and the tag table that names them.
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
//! `begin` first, before anything else, and exactly once — carrying
//! the arguments. Then every FUSE mount, each its own scope, and
//! every one answered before the next step. Then
//! trees, reads, writes and the family's own exchanges, as the server
//! pleases, in parallel and in any order.
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

mod client_request;

pub use client_request::*;

pub mod agents;
pub mod filesystem;
pub mod fuse;
pub mod tools;
