//! The OpenRouter API, as types.
//!
//! Ported faithfully from the objectiveai api crate's `openrouter`
//! module, with every `objectiveai_sdk` type its fields referenced
//! inlined — so the module is self-contained. Types only: the
//! transformation methods stayed behind, and what this container
//! does with these shapes is its own code's business.

pub mod request;
pub mod response;

mod serde_util;
