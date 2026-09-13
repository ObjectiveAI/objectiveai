//! The client side of a tree: what the server sends.
//!
//! [`request`] opens it — the paths to leave out, once.
//! [`channel_request`] closes it, and is the only other thing the
//! server ever sends on it: everything in between travels the other
//! way.
//!
//! # And, behind the `server` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::client::handle::Handle) to the proxy and
//! what to leave out, and get back the tree as a stream and the
//! handle that stops it. Every other module here is types only, and
//! this one still is unless the provider's server was asked for — the
//! same bargain [`server`](crate::server) makes.

pub mod channel_request;
pub mod request;

#[cfg(feature = "server")]
pub mod execute;
