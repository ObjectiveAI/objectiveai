//! Performing a mount, rather than describing it.
//!
//! [`execute`] opens the scope naming the path and the kind, waits for
//! the proxy to say the mount is made, and hands back an
//! [`ExecuteHandle`], through which the server answers the mount's
//! asks by the proxy's channel numbers, and the
//! [`Asks`](crate::container_proxy_endpoints::client::Asks) the mount
//! makes on its scope, each an [`Ask`] with no mount id — the scope is
//! the mount, and the server knows which.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod ask;
mod error;
mod execute;
mod execute_handle;

pub use ask::*;
pub use error::*;
pub use execute::*;
pub use execute_handle::*;
