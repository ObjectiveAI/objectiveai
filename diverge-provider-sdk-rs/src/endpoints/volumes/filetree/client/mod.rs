//! The client side of a volume filetree: what a client sends.
//!
//! [`request`] is the whole of it. A client asks and then listens; it
//! has nothing to send back, so there is no `response` here the way
//! there is on the other side.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it:
//! hand it a [`Handle`](crate::client::handle::Handle) and a
//! [`request::Frame`], get the tree.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes.

pub mod request;

#[cfg(feature = "client")]
pub mod execute;
