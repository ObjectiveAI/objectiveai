//! The channels a server opens on a caller for an MCP plugin.
//!
//! Two, and they are the same ask in different clothes: something the
//! provider cannot reach. One fetches the image, the other reaches the
//! database — see [`Frame`].
//!
//! What an OCI one carries is
//! [`http::request`](crate::shared::http::request), which is not here
//! and should not be: a tunneled HTTP request is the same request
//! whatever protocol rides it.

mod frame;

pub use frame::*;
