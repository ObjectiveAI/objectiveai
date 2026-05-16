//! Response types for vector completions.
//!
//! - [`unary`] - Complete (non-streaming) responses
//! - [`streaming`] - Incremental chunk-based responses
//! - [`Vote`] - Individual agent vote data

pub mod streaming;
pub mod unary;
mod vote;

pub use vote::*;
