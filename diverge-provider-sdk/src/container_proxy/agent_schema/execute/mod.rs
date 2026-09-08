//! Asking for the schema, rather than describing the ask.
//!
//! [`execute`] opens `/agent-schema` and hands back the one answer.
//! Its own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
