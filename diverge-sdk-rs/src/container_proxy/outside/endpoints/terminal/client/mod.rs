//! The client side of a terminal: what the server sends.
//!
//! [`request`] opens it at a size. [`channel_request`] is everything
//! the server puts in after: stdin, a resize, the stop. [`execute`]
//! performs it: the output as a stream, and the handle that writes.

pub mod channel_request;
pub mod execute;
pub mod request;
