//! The ObjectiveAI MCP plugin framework.
//!
//! Written on ONE assumption: a plugin runs inside ObjectiveAI. The
//! laboratory host gives it a container per completion, a single
//! connector, and its whole context in the environment — so the
//! framework reads that context once, directly, rather than making
//! every plugin rediscover it.

pub mod db;
mod environment;
pub use environment::*;
pub mod tools;

/// Re-exported so a plugin uses the SAME `rmcp` and `sqlx` the router
/// and pool came from. Depending on either separately risks two
/// versions in one binary, where a `Pool` or a `ToolRouter` from here
/// would not satisfy an API from there.
pub use {rmcp, sqlx};
