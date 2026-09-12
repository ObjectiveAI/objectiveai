//! Answering a command, rather than describing the answer.
//!
//! [`execute`] opens `/command/{channel}` and hands back an [`ExecuteHandle`]:
//! send each frame as it comes, then finish. Its own files are
//! flattened into it.

mod error;
mod execute;
mod execute_handle;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
