//! The daemon's Stores: the images it owns on each provider, as the
//! providers list them.
//!
//! An image is no record of the daemon's. What the daemon holds is the
//! `images::list` stream of each connected provider that keeps a Store,
//! for the connection's life — the provider serves one such listing a
//! connection — kept as a [`Mirror`], which [`watch`] fills and the
//! daemon's own `images list` reads, instead of asking the provider.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod mirror;
mod watch;

pub use mirror::*;
pub use watch::*;
