//! Enqueuing a message, rather than describing the ask.
//!
//! [`execute`] opens `/agent/enqueue`, sends the message, and waits
//! for its fate. Its own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
