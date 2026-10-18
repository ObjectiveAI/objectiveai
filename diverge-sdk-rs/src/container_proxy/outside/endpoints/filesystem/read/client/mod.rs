//! The client side of a read: what the server sends.
//!
//! [`request`] is the whole of it — one file, once.
//!
//! There is no `channel_request` and no `channel_response`: a read
//! is one question, and the server has nothing more to say to one
//! and nothing to answer on it.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::wire::client::handle::Handle) to the proxy and
//! the path, and get back the file's bytes as a stream. Every other
//! module here is types only, and this one still is unless the
//! provider's server was asked for — the same bargain
//! [`server`](crate::wire::server) makes.

pub mod request;

pub mod execute;
