//! Images answered from the other providers: a provider's asks on a
//! run scope — whether the caller holds an image, its manifest, its
//! blobs — answered by finding a connected provider that holds the
//! image and pulling from there, through the `images::oci` scope the
//! daemon keeps on it.
//!
//! The daemon holds no image itself. [`locate`] asks every other
//! connected provider at once whether it holds a digest, answers the
//! first yes, and lets the rest fill the daemon's memory of who holds
//! what; [`manifest`] and [`blob`] pull from a holder, the next when
//! one fails, and answer not held only when none is left; [`name`]
//! is the repository path the daemon answers a has with, which the
//! asking provider's own registry serves the image as and never
//! reads.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod locate;
mod name;
mod pull;

pub use locate::*;
pub use name::*;
pub use pull::*;
