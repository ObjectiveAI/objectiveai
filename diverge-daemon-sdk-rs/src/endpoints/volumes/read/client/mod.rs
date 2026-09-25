//! The client side of a volume read: what a client sends.
//!
//! [`request`] is the whole of it. A client asks and then listens; it
//! has nothing to send back, so there is no `response` here the way
//! there is on the other side.

pub mod request;