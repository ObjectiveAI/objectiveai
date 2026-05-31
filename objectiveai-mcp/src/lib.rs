//! ObjectiveAI MCP server library.
//!
//! Other crates can `use objectiveai_mcp::{ConfigBuilder, run}` and
//! spawn the server in-process; the binary at `main.rs` is a thin wrapper
//! that reads `Config` from the environment and calls [`run`].

mod format;
mod objectiveai;
mod run;

pub use run::*;
