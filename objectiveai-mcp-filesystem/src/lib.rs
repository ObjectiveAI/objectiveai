//! ObjectiveAI MCP filesystem library.
//!
//! Other crates can `use objectiveai_mcp_filesystem::{ConfigBuilder, run}`
//! and spawn the server in-process; the binary at `main.rs` is a thin
//! wrapper that reads `Config` from the environment and calls [`run`].

mod bash;
mod edit_file;
mod glob_search;
mod grep_search;
mod notebook;
mod read_file;
mod run;
mod state;
mod tools;
mod util;
mod write_file;

pub use run::*;
