//! The server side of an image push: what the daemon sends.
//!
//! [`response`] is the push's progress and the one answer that ends
//! it; [`channel_request`] is what the daemon asks the client for on
//! channels it opens — the provider's own asks, carried as they came.

pub mod channel_request;
pub mod response;
