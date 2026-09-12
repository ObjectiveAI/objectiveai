//! Asking the container's server, rather than describing the ask.
//!
//! [`execute`] opens `/tool/read-resource`, sends the params, and hands
//! back the one frame that answers — the result or the server's own
//! error, both the caller's to have. Its own files are flattened into
//! it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
