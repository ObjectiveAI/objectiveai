//! Answering a fuse mkdir, rather than describing the answer.
//!
//! [`execute`] opens `/fuse/mkdir/{channel}`, sends the one frame — or
//! nothing, the refusal — and closes. Its own files are flattened
//! into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
