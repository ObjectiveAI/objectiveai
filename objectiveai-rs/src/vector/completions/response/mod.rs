//! Response types for vector completions.
//!
//! - [`unary`] - Complete (non-streaming) responses
//! - [`streaming`] - Incremental chunk-based responses
//! - [`Vote`] - Individual LLM vote data
//! - [`Usage`] - Aggregated token and cost statistics

pub mod streaming;
pub mod unary;
mod usage;
mod vote;

pub use usage::*;
pub use vote::*;
