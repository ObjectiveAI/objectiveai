//! The caller half, behind the `client` feature.
//!
//! The mirror of [`server`](crate::server), and off for the same
//! reason: a provider should not have to compile a caller to answer a
//! frame.
//!
//! # Three levels, all empty
//!
//! [`connection_router`] reads the socket and routes by scope,
//! [`scope_router`] routes by channel, and [`channel_router`] is where
//! routing stops and a payload becomes somebody's stream. Each knows
//! only about the one below it.
//!
//! None of them holds a socket of its own.
//! [`Connection`](crate::connection::Connection) carries either kind,
//! so the reading and writing is not this module's to invent — and a
//! caller is not obliged to be the one that dialled. It may run a
//! server and be connected TO, which is the same connection with the
//! same frames on it.
//!
//! # What a caller supplies
//!
//! A handler, for the channels the far end opens: serving an image,
//! running a command, proxying Postgres. A caller is not only a source
//! of requests, so it is not only a client in the ordinary sense.
//!
//! # These are probably not public forever
//!
//! axum exposes a `Router` to register handlers on and a `serve` to
//! run it, and keeps the per-connection loop private. The same split
//! likely applies here: what a caller touches is a handle and a
//! handler, and how frames find their way is nobody else's business.
//! They are public now because nothing is inside them to hide.

pub mod channel_router;
pub mod connection_router;
pub mod scope_router;
