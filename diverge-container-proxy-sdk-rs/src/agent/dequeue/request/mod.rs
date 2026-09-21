//! The body: the key.
//!
//! The same JSON the provider protocol carries for a dequeue —
//! `{"key": "…"}` — so the proxy forwards it as it came.

pub use diverge_provider_sdk::shared::containers::dequeue::request::Request;
