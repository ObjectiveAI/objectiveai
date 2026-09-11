//! Registering the agent, rather than describing the ask.
//!
//! [`execute`] opens `/agent/register`, sends the agent, and reads the
//! one answer. Its own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
