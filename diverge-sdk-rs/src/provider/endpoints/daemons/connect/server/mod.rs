//! The server side of a connect: what the provider sends.
//!
//! [`response`] is what comes back on channel `0`: the connection
//! open, then the acceptor's frames, or the refusal.
//! [`channel_request`] is the one channel the provider opens, on
//! which the connector sends its frames. And the provider crate's `handle` answers the
//! request: the scope served whole, from the request to the finish.

pub mod channel_request;
pub mod response;
