//! The server side of a tool container connection: what a provider sends.
//!
//! [`channel_request`] is what it opens channels to ask the caller
//! for. [`channel_response`] answers the channels the caller opens
//! into the container. [`response`] is what comes back on channel
//! `0`. And, behind the `server` feature, [`handle`] answers the
//! request: the scope served whole, from the request to the finish.

pub mod channel_request;
pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
