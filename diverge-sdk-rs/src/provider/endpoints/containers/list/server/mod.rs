//! The server side of a listing: what a provider sends.
//!
//! [`response`] is what comes back on channel `0`, one per container.
//! And [`handle`] answers the request: the scope served whole, from
//! the request to the finish.

pub mod response;

pub mod handle;
