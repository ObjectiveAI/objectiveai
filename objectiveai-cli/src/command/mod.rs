//! Bare-naked CLI handlers mirroring the SDK's `cli::command` tree
//! 1-for-1. Each leaf file here pairs with an SDK leaf at the same
//! module path; the SDK leaf defines the typed `Request` / `Response` /
//! `ResponseItem` shapes, this side defines the `execute` / `execute_transform`
//! (and `execute_streaming` / `execute_streaming_transform` for chunk-or-id
//! leaves) that actually do the work.
//!
//! `run.rs` parses argv â†’ SDK `Command` â†’ SDK `Request` (via the SDK's
//! `TryFrom` impls), then dispatches to [`execute`] which fans out
//! through the tier `mod.rs` files to the leaves below.

pub mod agents;
pub mod api;
pub mod command;
pub mod db;
pub mod functions;
pub mod kill_all;
pub mod kill_helpers;
pub mod mcp;
pub mod plugins;
pub mod python;
pub mod reexec;
pub mod swarms;
pub mod tools;
pub mod update;
pub mod viewer;

pub use command::execute;
