//! The providers the daemon works with: one record per provider
//! identity.
//!
//! A record names the provider by the identity its token establishes,
//! and carries the [`Direction`](diverge_sdk::daemon::endpoints::providers::Direction)
//! its connection is made in, what the daemon announces itself with
//! on it, whether the daemon accepts other daemons and listens for
//! messages through it, whether it is enabled, a description, tags, when it was added and by
//! whom. A record an agent or a tool is pinned to, or mounts a volume
//! of, is not deleted — see [`in_use`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod providers;

pub use providers::*;
