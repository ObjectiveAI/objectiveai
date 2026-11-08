//! The client side of a listen: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client opens on it, the cancel: a listen has no end of its
//! own, so the client says when it is done.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! read the messages off the stream it hands back, and cancel it with
//! the cancel it hands back beside.

pub mod channel_request;
pub mod request;

pub mod execute;
