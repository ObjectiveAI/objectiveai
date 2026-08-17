//! The caller half, behind the `client` feature.
//!
//! The mirror of [`server`](crate::server), and off for the same
//! reason: a provider should not have to compile a caller to answer a
//! frame.
//!
//! # Empty
//!
//! Nothing here yet.
//!
//! What it will need already exists.
//! [`Connection`](crate::connection::Connection) carries either kind of
//! socket, so the reading and writing is not this module's to invent —
//! and a caller is not obliged to be the one that dialled. It may run
//! a server and be connected TO, which is the same connection with the
//! same frames on it.
