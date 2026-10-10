//! The server side of an accept: what the provider sends.
//!
//! [`response`] is what comes back on channel `0`: the daemon's
//! identity, or the refusal. [`channel_request`] is the one channel
//! the provider opens, the provider's half of a connection.
//! [`channel_response`] answers the daemon's half with the
//! connector's frames. And [`handle`] answers the request: the scope
//! served whole, from the request to the finish.

pub mod channel_request;
pub mod channel_response;
pub mod response;

pub mod handle;
