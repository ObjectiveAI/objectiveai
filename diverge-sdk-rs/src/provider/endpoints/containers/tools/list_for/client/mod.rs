//! The client side of a listing: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a lister opens on it, the stop: a listing has no end of
//! its own, so the lister says when it is done. A client sends
//! nothing else; there is no `response` here the way there is on the
//! other side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and the
//! request, read the listing off the stream it hands back, and stop
//! it with the stop it hands back beside.

pub mod channel_request;
pub mod request;

pub mod execute;
