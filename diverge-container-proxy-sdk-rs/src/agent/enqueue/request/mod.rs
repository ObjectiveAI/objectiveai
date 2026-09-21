//! The body: the message.
//!
//! The same JSON the provider protocol carries for an enqueue —
//! `{"key": "…", "content": [{"type": "text", "text": "…"}, …]}`,
//! the caller's key and MCP's content blocks — so the proxy forwards
//! it as it came.

pub use diverge_provider_sdk::shared::containers::enqueue::request::Request;
