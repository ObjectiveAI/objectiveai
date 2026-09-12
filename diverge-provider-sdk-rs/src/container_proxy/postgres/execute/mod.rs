//! Carrying a database connection, rather than describing it.
//!
//! [`execute`] opens `/postgres/{channel}` and hands back two things:
//! an [`ExecuteStream`] of what the container wrote, and an
//! [`ExecuteHandle`] to send what the database said. Its own files
//! are flattened into it.

mod error;
mod execute;
mod execute_handle;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
