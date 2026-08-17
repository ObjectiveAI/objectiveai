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
//! **Nothing is copied and nothing is rewritten.** A
//! [`Connection`](crate::connection::Connection) yields
//! [`Bytes`](bytes::Bytes), and the whole frame goes on as it arrived
//! — header included — for the cost of a refcount bump. The header is
//! read to know where the frame belongs and then left alone, because
//! a consumer needs it too: a finish is a frame, and telling one from
//! a response means reading the type.
//!
//! Bytes rather than a decoded frame, and that is forced rather than
//! clever. A decoded
//! [`ClientFrame`](crate::frame::client::ClientFrame) borrows from the
//! buffer it came from, so it cannot cross a channel at all. The far
//! side decodes.
//!
//! **Backpressure is not settled.** Unbounded queues let a slow
//! consumer grow memory without limit, and this protocol streams file
//! contents and image layers. Bounded ones stall the reader when one
//! fills, which is head-of-line blocking across every other scope on
//! the connection. Moving the queue does not fix it; a policy for what
//! a full one means does, and there is not one yet.
