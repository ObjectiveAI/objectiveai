//! One connection, and its frames routed by scope.
//!
//! Reads a [`Connection`](crate::connection::Connection), splits each
//! frame's header off, and hands the payload to whatever is waiting on
//! the scope it names.
//!
//! # Empty
//!
//! Nothing here yet. Two things about its shape are settled, and one
//! is not.
//!
//! **One hop, not two.** A registry keyed by scope AND channel, so a
//! payload goes straight to the receiver that wants it. Routing by
//! scope to a per-scope task, which then routes by channel, costs a
//! second wakeup and a second scheduler dispatch on every frame — a
//! hashmap lookup is tens of nanoseconds and a task hop is hundreds to
//! thousands. Registration is a hop, but it happens once per channel
//! rather than once per payload.
//!
//! **The payload is never copied.** A
//! [`Connection`](crate::connection::Connection) yields
//! [`Bytes`](bytes::Bytes), and
//! `bytes.slice(HEADER_LEN..)`([`HEADER_LEN`](crate::frame::HEADER_LEN))
//! hands on the same allocation with a refcount bump. That is also
//! forced rather than clever: a decoded
//! [`ClientFrame`](crate::frame::client::ClientFrame) borrows from the
//! buffer, so a frame cannot cross a channel — only its bytes can, and
//! the far side decodes.
//!
//! **Backpressure is not settled.** Unbounded queues let a slow
//! consumer grow memory without limit, and this protocol streams file
//! contents and image layers. Bounded ones stall the reader when one
//! fills, which is head-of-line blocking across every other scope on
//! the connection. Moving the queue does not fix it; a policy for what
//! a full one means does, and there is not one yet.
