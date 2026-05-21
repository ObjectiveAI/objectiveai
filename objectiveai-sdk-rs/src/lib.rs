//! ObjectiveAI SDK for Rust.
//!
//! This crate provides data structures, validation, and client-side compilation
//! for the ObjectiveAI API - a platform for scoring, ranking, and simulating
//! preferences using swarms of agents.
//!
//! # Core Concepts
//!
//! - **Agent**: A configured instance of a single upstream language model
//! - **Swarm**: A collection of Agents used together for voting
//! - **Vector Completion**: Runs multiple agents to vote on responses, producing weighted scores
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
//! - [`agent`] - Agent definitions, configuration, and completion APIs
//! - [`swarm`] - Swarm definitions and validation
//! - [`error`] - Error types
//! - [`functions`] - Function definitions, execution, and client-side compilation
//! - [`prefixed_uuid`] - UUID utilities
//! - [`vector`] - Vector completion APIs
//!
//! When the `http` feature is enabled:
//! - [`HttpClient`] - HTTP client for API requests
//! - [`HttpError`] - HTTP error types

pub mod agent;
pub mod arbitrary_util;
pub mod auth;
pub mod swarm;
pub mod error;
pub mod functions;
pub mod laboratories;
mod json_schema;
pub use json_schema::*;
pub mod prefixed_uuid;
mod remote;
pub(crate) mod serde_util;
pub mod vector;
mod weights;
mod util;

pub use remote::*;
pub use weights::*;

#[cfg(test)]
mod serde_util_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_util;

#[cfg(feature = "filesystem")]
pub mod filesystem;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
pub use http::*;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "viewer")]
pub mod viewer;

