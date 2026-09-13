//! The client side of a read: what the server sends.
//!
//! [`request`] is the whole of it — one file, once.
//!
//! There is no `channel_request` and no `channel_response`: a read
//! is one question, and the server has nothing more to say to one
//! and nothing to answer on it.

pub mod request;
