//! The server side of a volume list: what the daemon sends.
//!
//! [`response`] is the whole of it — the provider's, re-exported: the
//! answer means the same here. The daemon opens no channel of its
//! own.

pub use diverge_provider_sdk::endpoints::volumes::list::server::response;
