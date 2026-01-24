//! Ensemble LLM management, fetching, and retrieval.
//!
//! An Ensemble LLM is a fully-specified configuration of a single upstream LLM,
//! including model identity, prompt structure, decoding parameters, and output mode.

mod client;
/// Fetchers for retrieving Ensemble LLM definitions by ID.
pub mod fetcher;
/// Retrieval clients for listing Ensemble LLMs and getting usage statistics.
pub mod retrieval_client;

pub use client::*;
