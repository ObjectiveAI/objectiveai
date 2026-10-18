//! Performing a begin on a tool container, rather than describing it.
//!
//! [`execute`] opens the scope, waits for
//! [`Begun`](super::super::server::response::Frame::Begun), and hands
//! back four things: an [`ExecuteHandle`], through which the server
//! opens the five MCP exchanges, feeds the secrets and answers the
//! proxy's channels; the
//! [`Asks`](crate::container_proxy::outside::client::Asks) the proxy
//! opens on the scope, to relay; the [`Events`], which are the main
//! stream after `Begun` — the proxy's word whenever the container
//! goes active and inactive, and how its end is heard and told apart
//! from the connection dying; and what the program declared, a
//! [`Registered`](super::super::server::response::Registered).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod events;
mod execute;
mod execute_handle;

pub use error::*;
pub use events::*;
pub use execute::*;
pub use execute_handle::*;
