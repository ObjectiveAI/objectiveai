//! The `volumes` section of `config.yaml`.
//!
//! Two kinds of thing: the [`Store`]s new volumes are created in, each
//! with a capacity in bytes, and the [`Fixed`] volumes that exist
//! before any client asks, each offered to every identity or to those
//! a [hook](crate::hook) vouches for. [`Volumes`] holds both.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod fixed;
mod store;
mod volumes;

pub use fixed::*;
pub use store::*;
pub use volumes::*;
