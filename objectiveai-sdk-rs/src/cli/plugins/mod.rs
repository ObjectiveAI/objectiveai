//! Plugin protocol wire types and (future) helpers.

mod command;
mod output;

pub use command::*;
pub use output::*;

#[cfg(test)]
mod output_tests;
