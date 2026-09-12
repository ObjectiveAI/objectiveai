//! Subscribing, rather than describing the subscription.
//!
//! [`execute`] opens `/tool/notifications` and hands back an
//! [`ExecuteStream`] of what the container's server says. Its own
//! files are flattened into it.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
