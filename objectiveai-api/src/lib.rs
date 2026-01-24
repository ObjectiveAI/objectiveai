//! ObjectiveAI API server library.
//!
//! This crate provides the core implementation for the ObjectiveAI REST API,
//! which enables scoring, ranking, and simulating preferences using ensembles of LLMs.
//!
//! # Modules
//!
//! - [`auth`] - Authentication and API key management
//! - [`chat`] - Chat completions with Ensemble LLMs
//! - [`ctx`] - Request context and extensions
//! - [`ensemble`] - Ensemble management and retrieval
//! - [`ensemble_llm`] - Ensemble LLM management and retrieval
//! - [`error`] - Error response handling
//! - [`functions`] - Function execution and profile management
//! - [`util`] - Utility types for streaming and indexing
//! - [`vector`] - Vector completions for scoring and ranking

/// Authentication and API key management.
pub mod auth;
/// Chat completions with Ensemble LLMs.
pub mod chat;
/// Request context and extensions for dependency injection.
pub mod ctx;
/// Ensemble management, fetching, and retrieval.
pub mod ensemble;
/// Ensemble LLM management, fetching, and retrieval.
pub mod ensemble_llm;
/// Error response handling and conversion.
pub mod error;
/// Function execution, profile management, and computations.
pub mod functions;
/// Utility types for streaming and choice indexing.
pub mod util;
/// Vector completions for scoring and ranking responses.
pub mod vector;
