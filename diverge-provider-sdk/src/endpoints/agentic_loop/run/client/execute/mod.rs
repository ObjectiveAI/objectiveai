//! Performing the exchange, rather than describing it.
//!
//! [`execute`] starts the loop and answers the server's asks beside
//! it — the agent's tool calls, the provider's fetches and the
//! container's database connections alike; [`ExecuteStream`] is the
//! chunks coming out.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_stream;

pub use execute::*;
pub use execute_stream::*;
