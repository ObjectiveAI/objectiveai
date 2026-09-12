//! Making a mount, rather than describing the ask.
//!
//! [`execute`] opens `/fuse/mount`, sends the mount, and returns once
//! the proxy says it is serving. Its own files are flattened into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
