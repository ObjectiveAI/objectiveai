//! Vector completion client and supporting types.
//!
//! This module provides the client for creating vector completions, which
//! orchestrate multiple LLM chat completions for voting on response options.

/// Vote caching client for the global ObjectiveAI cache.
pub mod cache;
/// Fetcher for retrieving votes from the global cache.
pub mod cache_vote_fetcher;
mod client;
/// Fetcher for retrieving votes from historical completions.
pub mod completion_votes_fetcher;
mod error;
mod get_vote;
mod pfx;
mod response_key;
/// Usage tracking for vector completions.
pub mod usage_handler;
/// Vector response transformation utilities.
pub mod vector_responses;

pub use client::*;
pub use error::*;
pub use get_vote::*;
pub use pfx::*;
pub use response_key::*;
