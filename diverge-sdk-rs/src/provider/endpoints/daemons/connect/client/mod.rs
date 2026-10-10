//! The client side of a connect: what the connecting daemon sends.
//!
//! [`request`] opens the scope. [`channel_response`] answers the one
//! channel the provider opens, with the connector's client frames.
//! The connector opens no channel of its own.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) on the
//! provider and the request, get back a handle on the daemon reached
//! — a client of the daemon protocol, carried inside the scope.

pub mod channel_response;
pub mod request;

pub mod execute;
