//! The client side of a listing: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire. A client asks and
//! then listens; it has nothing to send back, so there is no `response`
//! here the way there is on the other side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and the
//! request, and read the containers off the stream it hands back.

pub mod request;

pub mod execute;
