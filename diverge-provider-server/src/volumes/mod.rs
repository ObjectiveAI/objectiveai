//! The volumes the provider offers: the SDK's `VolumeManager`,
//! supplied by this crate.
//!
//! [`Volumes`] is the handler, and [`Error`] what it fails with.
//! Nothing here is implemented yet: every method is the place its
//! implementation goes.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod volumes;

pub use error::*;
pub use volumes::*;
