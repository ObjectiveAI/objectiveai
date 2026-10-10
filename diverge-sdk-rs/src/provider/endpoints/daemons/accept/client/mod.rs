//! The client side of an accept: what the accepting daemon sends.
//!
//! [`request`] opens the scope. [`channel_request`] is what the daemon
//! opens on it: its half of each connection, and the stop.
//! [`channel_response`] answers the channel the provider opens, the
//! provider's half of each connection.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and an
//! [`Acceptor`](crate::provider::client::Acceptor), and every
//! connection the provider announces is answered through it.

pub mod channel_request;
pub mod channel_response;
pub mod request;

pub mod execute;
