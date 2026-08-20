//! Performing the exchange, rather than describing it.
//!
//! [`execute`] starts a plugin and serves the three kinds of
//! channel it opens; [`ExecuteHandle`] is the run itself.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;

pub use execute::*;
pub use execute_handle::*;
