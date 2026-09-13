//! Performing the exchange, rather than describing it.
//!
//! [`execute`] runs an agent container and hands back its id, an
//! [`ExecuteHandle`] — the scope held for the container's life, every
//! channel a caller may open into it, and the end of the run — and
//! the [`ExecuteStream`], the agent's conversation off the scope's main
//! stream. The channels the provider opens back are answered as they
//! come, through the caller's [`Answerers`](crate::client::Answerers).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod execute_stream;
mod filetree;
mod read;
mod write_path;

pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
pub use filetree::*;
pub use read::*;
pub use write_path::*;
