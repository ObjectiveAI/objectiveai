//! The server side of a volume write: what the daemon sends.
//!
//! [`response`] is what comes back on channel `0`, and
//! [`channel_request`] the one channel the daemon opens, for the
//! content — both the provider's, re-exported: a written answer and
//! the empty ask for the content are the same here.

pub use diverge_provider_sdk::endpoints::volumes::write::server::channel_request;
pub use diverge_provider_sdk::endpoints::volumes::write::server::response;
