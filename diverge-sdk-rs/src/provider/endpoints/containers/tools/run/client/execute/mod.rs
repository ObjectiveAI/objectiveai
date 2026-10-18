//! Performing the exchange, rather than describing it.
//!
//! [`execute`] runs a tool container and hands back its id, an
//! [`ExecuteHandle`] — the scope held for the container's life, every
//! channel a caller may open into it, the five MCP exchanges above
//! all, and the end of the run — and the [`ExecuteStream`], the
//! container's activity off the scope's main stream, one [`Event`] at
//! a time — the proxy's word that the container went active, or
//! inactive with its rotating secrets. The channels the provider
//! opens back are answered as they come, through the caller's
//! [`Answerers`](crate::provider::client::Answerers).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod event;
mod execute;
mod execute_handle;
mod execute_stream;
mod filetree;
mod read;
mod transfer;
mod write_path;

pub use event::*;
pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
pub use filetree::*;
pub use read::*;
pub use transfer::*;
pub use write_path::*;
