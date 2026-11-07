//! The client side of a frontend request: what the server sends.
//!
//! [`request`] opens it with the request's head. [`channel_request`]
//! is everything the server puts in after: the body, its end, the
//! cancel. [`execute`] performs it: the answer as a stream, and the
//! handle that writes.

pub mod channel_request;
pub mod execute;
pub mod request;
