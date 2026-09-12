//! Clearing the queue, rather than describing the ask.
//!
//! [`execute`] opens `/agent/dequeue` and reads the one answer. Its
//! own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
