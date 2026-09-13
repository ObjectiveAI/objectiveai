//! Performing a begin on a tool container, rather than describing it.
//!
//! [`execute`] opens the scope, waits for
//! [`Begun`](super::super::server::response::Frame::Begun), and hands
//! back an [`ExecuteHandle`], through which the server opens the five
//! MCP exchanges and answers the proxy's channels, and the
//! [`Asks`](crate::container_proxy_endpoints::client::Asks) the proxy
//! opens on the scope, to relay. Nothing rides the main stream after
//! `Begun` on a tool container, so nothing is handed back for it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_handle;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
