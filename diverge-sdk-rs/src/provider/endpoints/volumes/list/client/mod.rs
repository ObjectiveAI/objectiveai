//! The client side of a volume listing: what a client sends.
//!
//! [`request`] opens the scope, and it is empty; [`channel_request`]
//! is the one channel a caller opens on it, the stop: a listing has
//! no end of its own, so the caller says when it is done. A client
//! sends nothing else; there is no `response` here the way there is
//! on the other side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], read the listing off the stream it hands
//! back, and stop it with the stop it hands back beside.
//!
//! Every other module in [`endpoints`](crate::provider::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::wire::client) itself makes.

pub mod channel_request;
pub mod request;

pub mod execute;
