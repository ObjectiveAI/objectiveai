//! The client side of a capacity request: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire, and it carries
//! nothing: the question has no parameters, because what is asked is
//! the whole of what the caller may use, and the caller is the
//! connection's identity.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle), get the answer
//! back.

pub mod request;

pub mod execute;
