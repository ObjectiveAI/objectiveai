//! The volumes the provider offers: the SDK's `VolumeManager`,
//! supplied by this crate.
//!
//! [`VolumeManager`] is the handler, holding the stores it may create
//! volumes in and the fixed volumes it holds already, as the `volumes`
//! section of the configuration names them; [`Error`] is what it
//! fails with. Nothing here is implemented yet: every method is the
//! place its implementation goes.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod volume_manager;

pub use error::*;
pub use volume_manager::*;
