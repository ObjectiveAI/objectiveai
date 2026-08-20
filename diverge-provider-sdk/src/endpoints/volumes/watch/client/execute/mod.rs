//! Performing the exchange, rather than describing it.
//!
//! [`execute`] starts the watch; [`ExecuteStream`] is what the
//! provider says about the tree, and dropping it says stop.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_stream;

pub use execute::*;
pub use execute_stream::*;
