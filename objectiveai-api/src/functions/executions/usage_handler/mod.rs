//! Usage handling for Function executions.
//!
//! Provides traits and implementations for recording usage after
//! Function execution completes.

mod log_usage_handler;
mod usage_handler;

pub use log_usage_handler::*;
pub use usage_handler::*;
