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
//! # Empty
//!
//! Nothing here yet. The parts that are not a caller's alone have gone
//! elsewhere: [`Connection`](crate::connection::Connection) carries
//! either kind of socket, and [`router`](crate::router) is the read
//! loop that both halves need. What is left for this module is what
//! only a caller does — open scopes, and answer the channels a
//! provider opens back.
