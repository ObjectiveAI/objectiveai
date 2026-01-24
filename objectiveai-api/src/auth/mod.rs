//! Authentication and API key management.
//!
//! This module provides traits and implementations for managing API keys,
//! BYOK (Bring Your Own Key) OpenRouter keys, and credit balances.

mod client;
mod objectiveai;

pub use client::*;
pub use objectiveai::*;
