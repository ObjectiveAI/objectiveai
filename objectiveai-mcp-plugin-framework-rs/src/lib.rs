//! The ObjectiveAI MCP plugin framework.
//!
//! Written on ONE assumption: a plugin runs inside ObjectiveAI. The
//! laboratory host gives it a container per completion, a single
//! connector, and its whole context in the environment — so the
//! framework reads that context once, directly, rather than making
//! every plugin rediscover it.

pub mod command_executor;
pub use command_executor::command_executor;
pub mod config;
pub mod db;
mod environment;
pub use environment::*;
pub mod serve;
pub mod tools;

/// Re-exported so a plugin uses the SAME `objectiveai_sdk`, `rmcp` and
/// `sqlx` the executor, router and pool came from. Depending on any of
/// them separately risks two versions in one binary, where a `Pool`, a
/// `ToolRouter` or a `CommandExecutor` from here would not satisfy an
/// API from there.
///
/// The SDK matters most: every `cli::command::*::execute` is generic
/// over the `CommandExecutor` TRAIT, so a separately-resolved SDK
/// makes [`command_executor`]'s return type implement a different
/// trait of the same name, and the call simply will not compile.
pub use {objectiveai_sdk, rmcp, sqlx};
