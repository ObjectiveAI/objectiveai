//! Response types for function executions.
//!
//! - [`unary`] - Complete (non-streaming) responses
//! - [`streaming`] - Incremental chunk-based responses

pub mod streaming;
pub mod unary;
