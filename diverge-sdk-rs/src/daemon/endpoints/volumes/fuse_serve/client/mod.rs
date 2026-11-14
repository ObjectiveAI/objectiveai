//! The client side of a volume FUSE serve: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is what a
//! client opens on it once it is served: the asks, the file tree and
//! the cancel.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! and get the served scope, held.

pub mod channel_request;
pub mod request;

pub mod execute;
