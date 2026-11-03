//! The client side of a discover request: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire, and it carries
//! nothing beyond its tag. [`execute`] performs the exchange: hand it a
//! [`Handle`](crate::wire::client::handle::Handle), get the answer back.

pub mod request;

pub mod execute;
