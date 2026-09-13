//! The server side of an agent container begin: what the proxy
//! sends.
//!
//! [`channel_request`] is what it opens channels to ask the server
//! for. [`channel_response`] answers the channels the server opens
//! into the container. [`response`] is what comes back on channel
//! `0`.

pub mod channel_request;
pub mod channel_response;
pub mod response;
