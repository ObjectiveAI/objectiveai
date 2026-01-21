//! ObjectiveAI SDK for Rust.
//!
//! This crate provides data structures, validation, and client-side compilation
//! for the ObjectiveAI API - a platform for scoring, ranking, and simulating
//! preferences using ensembles of LLMs.
//!
//! # Core Concepts
//!
//! - **Ensemble LLM**: A configured instance of a single upstream language model
//! - **Ensemble**: A collection of Ensemble LLMs used together for voting
//! - **Vector Completion**: Runs multiple LLMs to vote on responses, producing weighted scores
//! - **Function**: A composable scoring pipeline built from Vector Completions
//! - **Profile**: Learned weights for a Function, trained on example data
//!
//! # Features
//!
//! - `http` (default): Enables the HTTP client for making API requests
//!
//! # Modules
//!
//! - [`auth`] - API authentication types
//! - [`chat`] - Chat completion APIs
//! - [`ensemble`] - Ensemble definitions and validation
//! - [`ensemble_llm`] - Ensemble LLM configurations
//! - [`error`] - Error types
//! - [`functions`] - Function definitions, execution, and client-side compilation
//! - [`prefixed_uuid`] - UUID utilities
//! - [`vector`] - Vector completion APIs
//!
//! When the `http` feature is enabled:
//! - [`HttpClient`] - HTTP client for API requests
//! - [`HttpError`] - HTTP error types

pub mod auth;
pub mod chat;
pub mod ensemble;
pub mod ensemble_llm;
pub mod error;
pub mod functions;
pub mod prefixed_uuid;
pub mod vector;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
