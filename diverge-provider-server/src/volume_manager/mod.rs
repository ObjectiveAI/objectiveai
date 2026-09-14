//! The volumes the provider offers: the SDK's `VolumeManager`,
//! supplied by this crate.
//!
//! [`VolumeManager`] is the handler, holding the stores it may create
//! volumes in and the fixed volumes it holds already, as the `volumes`
//! section of the configuration names them, and the [`Cache`] of what
//! it knows about them between calls; [`Error`] is what it fails with.
//! No trait method is implemented yet: each is the place its
//! implementation goes, and the cache is what it will serve from.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cache;
mod error;
mod volume_manager;

pub use cache::*;
pub use error::*;
pub use volume_manager::*;
