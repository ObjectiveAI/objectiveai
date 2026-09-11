//! Hearing the container's asks, rather than describing them.
//!
//! [`execute`] takes `/requests` and hands back an [`ExecuteStream`]
//! of [`Ask`]s, one per frame the container sends, for as long as the
//! connection lives. Its own files are flattened into it.

mod ask;
mod error;
mod execute;
mod execute_stream;

pub use ask::*;
pub use error::*;
pub use execute::*;
pub use execute_stream::*;
