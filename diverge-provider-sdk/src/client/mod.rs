//! The caller half, behind the `client` feature.
//!
//! The mirror of [`server`](crate::server), and off for the same
//! reason: a provider should not have to compile a caller to answer a
//! frame.
//!
//! # What a caller supplies
//!
//! A handler, for the channels the far end opens: serving an image,
//! running a command, proxying Postgres. A caller is not only a source
//! of requests, so it is not only a client in the ordinary sense.
//!
//! # What is here
//!
//! One socket, split in two.
//!
//! [`router`] is the read loop: frames off the socket, forwarded to
//! whoever is waiting. [`handle`] is the write half and what a caller
//! keeps hold of. They are separate because a
//! [`Sink`](futures_util::Sink) needs `&mut` and a read loop never
//! finishes, so one type holding both could only ever do one of them.
//!
//! Two channels run between them, both unbounded: registrations
//! forward, saying where frames should go before the request that
//! causes them, and closures back, saying an entry is gone.
//!
//! The socket itself is not here.
//! [`Connection`](crate::connection::Connection) carries either kind
//! under either half, because which end dialled is not a fact about
//! the protocol.

pub mod handle;
pub mod router;
