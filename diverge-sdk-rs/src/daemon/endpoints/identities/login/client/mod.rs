//! The client side of an identities login: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client opens on it, the cancel. A client sends nothing
//! else; there is no `response` here the way there is on the other
//! side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! read the answers off the stream it hands back, and cancel it with the
//! cancel it hands back beside.

pub mod channel_request;
pub mod request;

pub mod execute;
