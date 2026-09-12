//! Answering a tool listing, rather than describing the answer.
//!
//! [`execute`] opens `/mcp/list-tools/{channel}`, sends the one frame — or nothing,
//! the refusal — and closes. Its own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
