//! The body: the key.
//!
//! The same JSON the provider protocol carries for a dequeue —
//! `{"key": "…"}` — so the proxy forwards it as it came.

pub use crate::shared::containers::dequeue::request::Request;
