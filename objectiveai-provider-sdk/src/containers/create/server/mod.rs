//! The server side of a container creation: what a provider sends.
//!
//! [`channel_request`] is what it opens channels to ask the caller for
//! while pulling the image. [`response`] is the answer to the creation
//! itself, on channel `0`.
//!
//! Under construction — [`response`] is still empty.

pub mod channel_request;
pub mod response;
