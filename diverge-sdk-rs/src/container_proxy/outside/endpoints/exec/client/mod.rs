//! The client side of an exec: what the server sends.
//!
//! [`request`] opens it — the command, once. [`channel_request`] cancels
//! it, killing the command, and is the only other thing the server
//! ever sends on it. [`execute`] performs the exchange: the output as
//! a stream, and the handle that cancels it.

pub mod channel_request;
pub mod execute;
pub mod request;
