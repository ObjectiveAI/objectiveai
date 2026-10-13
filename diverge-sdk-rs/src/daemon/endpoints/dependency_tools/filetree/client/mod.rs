//! The client side of a filetree: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client may open on it, the cancel. A client sends nothing
//! else; there is no `response` here the way there is on the other
//! side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! and get the daemon's answer.

pub mod channel_request;
pub mod request;

pub mod execute;
