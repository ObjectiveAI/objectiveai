//! Performing the exchange, rather than describing it.
//!
//! [`execute`] joins a tool container somebody else is running and
//! hands back an [`ExecuteHandle`]: the scope held for the
//! connection's life, every channel a connector may open into the
//! container, and the connection's end. The one channel the provider
//! opens back on a connector — the content of a write the connector
//! started — is answered from the handle's own pending writes.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod filetree;
mod read;
mod serve;
mod write_path;

pub use execute::*;
pub use execute_handle::*;
pub use filetree::*;
pub use read::*;
pub use write_path::*;
