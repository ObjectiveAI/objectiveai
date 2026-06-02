//! Async handlers for every cli leaf command (except `api`,
//! `schemas`, the internal `instance` subprocess runner, and
//! clap's `external` plugin dispatch; `logs` is also skipped
//! for now). Stubs today; typed `Args` + `pub async fn
//! handle(...)` signatures land in follow-up commits.

mod into_command;
pub use into_command::*;

mod ok;
pub use ok::*;

pub mod agents;
pub mod functions;
pub mod mcp;
pub mod plugins;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod viewer;
