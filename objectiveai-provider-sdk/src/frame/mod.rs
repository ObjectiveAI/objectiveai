//! The wire frame.
//!
//! Every message is one WebSocket BINARY frame:
//!
//! ```text
//! [type: u8][scope: varint][channel: varint][payload…]
//! ```
//!
//! `type` leads because it is the only fixed-width field: a reader
//! knows what the frame is before parsing anything of variable length.
//! No length prefix — WebSocket already delimits messages, so carrying
//! one would be paying twice for the same fact.
//!
//! # Scopes and channels
//!
//! A **scope** is one client request and everything that follows from
//! it. The client does not choose it: it sends a request with no
//! scope, and the server mints one in its ack. Every later frame in
//! either direction carries it.
//!
//! A **channel** is one exchange inside a scope. Channel `0` is the
//! answer to the client's own request. Any other channel is a request
//! the SERVER made, and only the server opens them — which is why the
//! two ends can never collide over one.
//!
//! # Types
//!
//! `type` means different things by direction, which is why
//! [`ClientFrame`](client::ClientFrame) and
//! [`ServerFrame`](server::ServerFrame) are separate types rather than
//! one shape with fields that are sometimes ignored.
//!
//! | type | client sends | server sends |
//! |------|--------------|--------------|
//! | 0    | a new request, if unscoped; otherwise an ack | ack |
//! | 1    | body | body |
//! | 2    | finish | finish |
//! | 3+   | — | a request, opening a new channel |
//!
//! Ack / body / finish is one sequence used in both directions: the
//! server answering the client's request on channel `0`, and the
//! client answering a server request on that request's channel. The
//! values above `2` are what a server request IS, and belong to the
//! protocol being carried rather than to this layer.

pub mod client;
pub mod server;
