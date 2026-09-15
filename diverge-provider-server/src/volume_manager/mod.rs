//! The volumes the provider offers: the SDK's `VolumeManager`,
//! supplied by this crate.
//!
//! [`VolumeManager`] is the handler, holding the stores it may create
//! volumes in and the fixed volumes it holds already, as the `volumes`
//! section of the configuration names them, and the [`Cache`] of what
//! it knows about them between calls; [`Handle`] is the SDK's
//! `Volume`, one cache entry shared with the cache, whose lock the
//! entry carries; [`Error`] is what both fail with. `get` and the
//! lock are implemented; every other trait method is the place its
//! implementation goes, and the cache is what it will serve from.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cache;
mod error;
mod handle;
mod volume_manager;

pub use cache::*;
pub use error::*;
pub use handle::*;
pub use volume_manager::*;
