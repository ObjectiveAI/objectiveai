//! The client side of a filesystem listing: what a client sends.
//!
//! [`request`] is the whole of it, and it is empty. A client asks and
//! then listens; it has nothing to send back, so there is no
//! `response` here the way there is on the other side.

pub mod request;
