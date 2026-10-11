//! The `images` section of `config.yaml`.
//!
//! One thing: the [`Images`] store, where the images clients build
//! on the provider are kept — a podman storage root of its own,
//! apart from the cache of images the provider pulls, with a
//! capacity in bytes and nothing ever evicted.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod images;

pub use images::*;
