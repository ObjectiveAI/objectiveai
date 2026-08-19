//! The client side of a watch: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire — one name, once.
//! Everything after it travels the other way.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] starts the watch and hands back a
//! [`ScopeResponseStream`](crate::client::scope_response_stream::ScopeResponseStream)
//! of what the provider says about the tree. It is the one volume
//! endpoint that does not collapse into a single answer, because a
//! watch does not end.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes. It is flattened in rather
//! than kept behind a name of its own, because a name for it would be
//! a name for one function.

pub mod request;

#[cfg(feature = "client")]
mod execute;

#[cfg(feature = "client")]
pub use execute::*;
