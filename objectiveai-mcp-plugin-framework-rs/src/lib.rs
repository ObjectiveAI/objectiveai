//! The ObjectiveAI MCP plugin framework.
//!
//! Written on ONE assumption: a plugin runs inside ObjectiveAI. The
//! laboratory host gives it a container per completion, a single
//! connector, and its identity in the environment — so the framework
//! reads that context once, directly, rather than making every plugin
//! rediscover it.

pub mod identity;

pub use identity::{Arguments, Environment, environment};
