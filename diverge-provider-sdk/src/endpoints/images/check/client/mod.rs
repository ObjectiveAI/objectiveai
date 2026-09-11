//! The client side of an image check: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire. A client asks and
//! then listens; it has nothing to send back, so there is no `response`
//! here the way there is on the other side.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::client::handle::Handle) and a
//! [`request::Frame`], get an answer back. Every other module in
//! [`endpoints`](crate::endpoints) is types only, and this one still is
//! unless a caller asked for the half of the crate that can hold a
//! socket — the same bargain [`client`](crate::client) itself makes.
//!
//! It is flattened in rather than kept behind a module of its own,
//! because a name for it would be a name for one function. What a
//! caller reaches for sits beside the `request::Frame` it takes.

pub mod request;

#[cfg(feature = "client")]
pub mod execute;
