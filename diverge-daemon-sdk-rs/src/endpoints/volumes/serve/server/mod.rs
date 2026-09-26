//! The server side of a volume serve: what the daemon sends.
//!
//! Both are the provider's, re-exported: [`response`] says the volume
//! is served, or why not, and [`channel_response`] answers each ask
//! with the shared vocabulary's own frame. The daemon opens no
//! channel of its own.

pub use diverge_provider_sdk::endpoints::volumes::serve::server::channel_response;
pub use diverge_provider_sdk::endpoints::volumes::serve::server::response;
