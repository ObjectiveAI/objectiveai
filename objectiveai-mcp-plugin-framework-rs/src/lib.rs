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

/// Re-exported so a plugin uses the SAME `sqlx` the pool came from.
/// Depending on it separately risks two versions in one binary, where
/// a `Pool` from here would not satisfy a query API from there.
pub use sqlx;
