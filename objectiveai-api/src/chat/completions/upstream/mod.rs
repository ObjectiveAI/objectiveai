//! Upstream provider clients for LLM inference.
//!
//! This module contains clients for communicating with upstream LLM providers
//! like OpenRouter.

mod client;
mod error;
/// OpenRouter provider client and types.
pub mod openrouter;
mod params;
mod upstream;

pub use client::*;
pub use error::*;
pub use params::*;
pub use upstream::*;
