//! Ensemble management, fetching, and retrieval.
//!
//! Ensembles are collections of Ensemble LLMs used together for voting.
//! This module provides clients for listing, retrieving, and fetching ensembles.

mod client;
/// Fetchers for retrieving ensemble definitions by ID.
pub mod fetcher;
/// Retrieval clients for listing ensembles and getting usage statistics.
pub mod retrieval_client;

pub use client::*;
