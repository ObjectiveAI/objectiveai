//! The channels a client opens on a serve scope: the nine asks, and
//! the stop. See [`Frame`].
//!
//! The asks are the proxy's own frames — the ones a mount opens on
//! its scope, with no mount id, because the scope is the volume as
//! the mount scope is the mount — re-exported here so that a caller
//! bridging a mount to a volume forwards each ask as it is:
//! [`Path`], [`Read`], [`Write`], [`Rename`], [`Truncate`] and
//! [`Setattr`] are theirs.

mod frame;

pub use frame::*;
pub use crate::container_proxy_endpoints::fuse::mount::server::channel_request::{Path, Read, Rename, Setattr, Truncate, Write};
