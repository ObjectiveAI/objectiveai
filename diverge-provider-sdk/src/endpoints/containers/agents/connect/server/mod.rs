//! The server side of an agent container connection: what a provider sends.
//!
//! [`channel_request`] is what it opens channels to ask the caller
//! for. [`channel_response`] answers the channels the caller opens
//! into the container. [`response`] is what comes back on channel
//! `0`.

pub mod channel_request;
pub mod channel_response;
pub mod response;
