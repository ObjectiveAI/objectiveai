//! Vector completion client and supporting types.
//!
//! This module provides the client for creating vector completions, which
//! orchestrate multiple LLM chat completions for voting on response options.

mod client;
mod error;
mod get_vote;
mod pfx;
mod response_key;
/// Usage tracking for vector completions.
pub mod usage_handler;
pub mod vector_responses;

pub use client::*;
pub use error::*;
pub use get_vote::*;
pub use pfx::*;
pub use response_key::*;
