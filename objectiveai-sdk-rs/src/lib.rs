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
pub mod data_url;
pub mod error;
pub mod functions;
mod json_schema;
pub mod laboratories;
pub mod machine;
pub mod swarm;
pub use json_schema::*;
pub mod prefixed_uuid;
mod remote;
pub(crate) mod serde_util;
mod util;
pub mod vector;
mod weights;

pub use remote::*;
pub use weights::*;

#[cfg(test)]
mod serde_util_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_util;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
pub use http::*;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "lockfile")]
pub mod lockfile;

#[cfg(windows)]
pub(crate) mod win_handles;

// Every feature whose code spawns a subprocess calls
// `process::no_window`, so the module exists for their union — not
// just the lock/reaper features whose deps back `kill_pid` (that
// function carries its own narrower gate).
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "cli-executor",
    feature = "cli-listener",
    feature = "lockfile",
    feature = "subprocess-reaper"
))]
pub mod process;

#[cfg(feature = "subprocess-reaper")]
pub mod subprocess_reaper;

// `client_objectiveai_mcp` is the reverse-attach protocol's wire
// envelope. The typed `server_request::Payload` and
// `server_response::Payload` variants reference `mcp::tool::*` /
// `mcp::resource::*` shapes, so the module is gated behind the same
// feature. Pure-`http` consumers (no MCP) wouldn't have anything to
// do with it anyway — the McpHandler trait + WS streaming path also
// require this module.
#[cfg(feature = "mcp")]
pub mod client_objectiveai_mcp;

#[cfg(feature = "cli")]
pub mod cli;
