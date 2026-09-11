//! Performing the exchange, rather than describing it.
//!
//! [`execute`] starts the watch and hands back two things:
//! [`ExecuteStream`], what the provider says about the tree, and
//! [`ExecuteHandle`], which is how a caller stops it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod execute_stream;

pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
