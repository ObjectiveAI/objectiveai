//! Performing a begin on an agent container, rather than describing
//! it.
//!
//! [`execute`] opens the scope, waits for
//! [`Begun`](super::super::server::response::Frame::Begun), and hands
//! back three things: an [`ExecuteHandle`], through which the server
//! opens the family's own channels and answers the proxy's; the
//! [`Asks`](crate::container_proxy_endpoints::client::Asks) the proxy
//! opens on the scope, to relay; and the [`Chunks`], the agent's
//! conversation off the main stream.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod chunks;
mod error;
mod execute;
mod execute_handle;

pub use chunks::*;
pub use error::*;
pub use execute::*;
pub use execute_handle::*;
