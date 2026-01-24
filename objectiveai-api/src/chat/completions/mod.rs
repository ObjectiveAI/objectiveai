//! Chat completions client for generating text responses.

mod client;
mod error;
/// Upstream provider clients (e.g., OpenRouter).
pub mod upstream;
/// Usage tracking handlers.
pub mod usage_handler;

pub use client::*;
pub use error::*;
