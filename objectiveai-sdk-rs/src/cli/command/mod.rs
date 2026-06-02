//! Async handlers for every cli leaf command (except `api`,
//! `schemas`, the internal `instance` runner, and clap's
//! `external` plugin dispatch). Stubs today; typed `Args` +
//! `pub async fn handle(...)` signatures land in follow-up
//! commits.
pub mod agents;
pub mod functions;
pub mod logs;
pub mod mcp;
pub mod plugins;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod vector;
pub mod viewer;
