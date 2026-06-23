//! ObjectiveAI MCP server library.
//!
//! Other crates can `use objectiveai_mcp::{ConfigBuilder, run}` and
//! spawn the server in-process; the binary at `main.rs` is a thin wrapper
//! that reads `Config` from the environment and calls [`run`].

pub mod agent_args_registry;
mod format;
mod header_session_manager;
pub mod objectiveai;
mod run;

#[cfg(test)]
mod format_tests;

pub use run::*;
