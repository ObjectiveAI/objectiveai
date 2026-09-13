//! The client side of a tree: what the server sends.
//!
//! [`request`] opens it — the paths to leave out, once.
//! [`channel_request`] closes it, and is the only other thing the
//! server ever sends on it: everything in between travels the other
//! way.

pub mod channel_request;
pub mod request;
