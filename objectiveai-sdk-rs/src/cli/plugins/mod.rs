//! Plugin protocol wire types and (future) helpers.

mod error;
mod mcp;
mod output;

pub use error::*;
pub use mcp::*;
pub use output::*;

#[cfg(test)]
mod output_tests;
