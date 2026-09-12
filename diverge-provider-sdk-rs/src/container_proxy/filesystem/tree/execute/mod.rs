//! Watching the tree, rather than describing the watch.
//!
//! [`execute`] opens `/filesystem/tree` and hands back an [`ExecuteStream`]
//! of filetree events, snapshot first. Its own files are flattened
//! into it.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
