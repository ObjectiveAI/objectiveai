//! ObjectiveAI MCP filesystem library.
//!
//! Other crates can `use objectiveai_mcp_laboratory::{ConfigBuilder, run}`
//! and spawn the server in-process; the binary at `main.rs` is a thin
//! wrapper that reads `Config` from the environment and calls [`run`].

mod bash;
mod composite;
mod run;
mod tools;
mod transfer;

pub use run::*;
