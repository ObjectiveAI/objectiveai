//! Performing the exchange, rather than describing it.
//!
//! [`execute`] runs a tool container and hands back its id and an
//! [`ExecuteHandle`]: the scope held for the container's life, every
//! channel a caller may open into it — the five MCP exchanges above
//! all — and the end of the run. The channels the provider opens back
//! are answered as they come, through the caller's
//! [`Answerers`](crate::client::Answerers).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod filetree;
mod read;
mod transfer;
mod write_path;

pub use execute::*;
pub use execute_handle::*;
pub use filetree::*;
pub use read::*;
pub use transfer::*;
pub use write_path::*;
