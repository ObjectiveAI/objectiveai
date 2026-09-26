//! The client side of a volume serve: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] is what the
//! client opens on it — the provider's, re-exported: the nine asks
//! and the stop mean the same here, the volume being the daemon's.

pub mod request;

pub use diverge_provider_sdk::endpoints::volumes::serve::client::channel_request;
