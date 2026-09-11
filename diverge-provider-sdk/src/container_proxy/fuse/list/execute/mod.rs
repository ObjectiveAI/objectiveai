//! Answering a fuse listing, rather than describing the answer.
//!
//! [`execute`] opens `/fuse/list/{channel}`, sends the one frame — or
//! nothing, the refusal — and closes. Its own files are flattened
//! into it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
