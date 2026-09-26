//! The client side of a version request: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire, and it carries
//! nothing: the question has no parameters, because there is nothing to
//! ask about beyond the asking.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle), get a version back.
//! Every other module in [`endpoints`](crate::provider::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::wire::client) itself makes.

pub mod request;

pub mod execute;
