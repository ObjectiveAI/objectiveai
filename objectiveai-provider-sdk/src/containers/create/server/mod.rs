//! The server side of a container creation: what a provider sends.
//!
//! [`channel_request`] is what it opens channels to ask the caller for
//! while pulling the image. [`channel_response`] answers the channels
//! the caller opens into the container. [`response`] is what comes
//! back on channel `0` once it runs.

pub mod channel_request;
pub mod channel_response;
pub mod response;
