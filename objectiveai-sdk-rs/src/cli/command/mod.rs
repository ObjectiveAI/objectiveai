//! Async handlers for every cli leaf command (except `api`,
//! `schemas`, and clap's `external` plugin dispatch;
//! `logs` is also skipped for now). Stubs today; typed
//! `Args` + `pub async fn handle(...)` signatures land in
//! follow-up commits.

mod into_command;
pub use into_command::*;

pub mod agents;
pub mod functions;
pub mod instance;
pub mod mcp;
pub mod plugins;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod viewer;
