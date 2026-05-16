//! Response types for function executions.
//!
//! - [`unary`] - Complete (non-streaming) responses
//! - [`streaming`] - Incremental chunk-based responses

mod output;
pub mod streaming;
pub mod unary;

pub use output::*;
