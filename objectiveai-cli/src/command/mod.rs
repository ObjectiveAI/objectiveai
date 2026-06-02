//! Bare-naked CLI handlers mirroring the SDK's `cli::command` tree
//! 1-for-1. Each leaf file here pairs with an SDK leaf at the same
//! module path; the SDK leaf defines the typed `Request` / `Response` /
//! `ResponseItem` shapes, this side defines the `execute` / `execute_jq`
//! (and `execute_streaming` / `execute_streaming_jq` for chunk-or-id
//! leaves) that actually do the work.
//!
//! `run.rs` parses argv → SDK `Command` → SDK `Request` (via the SDK's
//! `TryFrom` impls), then dispatches to [`execute`] which fans out
//! through the tier `mod.rs` files to the leaves below.

pub mod agents;
pub mod command;
pub mod config;
pub mod functions;
pub mod logs;
pub mod mcp;
pub mod plugins;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod viewer;

pub use command::execute;
