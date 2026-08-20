//! Performing the exchange, rather than describing it.
//!
//! [`execute`] starts the loop and answers the agent's tool
//! calls beside it; [`ExecuteStream`] is the chunks coming out.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_stream;

pub use execute::*;
pub use execute_stream::*;
