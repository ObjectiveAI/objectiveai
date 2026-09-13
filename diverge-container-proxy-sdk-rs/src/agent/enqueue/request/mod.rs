//! The body: the message.
//!
//! The same JSON the provider protocol carries for an enqueue —
//! `{"prompt": "…"}` — so the proxy forwards it as it came.

pub use diverge_provider_sdk::shared::containers::enqueue::request::Request;
