//! Running the loop, rather than describing the run.
//!
//! [`execute`] opens `/agent/run`, hands over the request, and hands
//! back an [`ExecuteStream`] of the loop's chunks. Its own files are
//! flattened into it.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
