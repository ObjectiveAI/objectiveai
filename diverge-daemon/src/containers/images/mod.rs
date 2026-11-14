//! Images answered from the other providers and the remote daemons: a
//! provider's asks on a run scope, and a client's on the daemon's own
//! `images::read` scope — whether an image can be supplied, its
//! manifests, its blobs, the proof, its document — answered by finding
//! a source that can supply the image, and reading from there, through
//! the one `images::read` scope that answered held.
//!
//! The daemon holds no image itself. [`locate`] asks every provider and
//! every remote daemon in reach at once whether it has the image, takes
//! the first yes and cancels every other; [`manifest`], [`blob`] and
//! [`prove`] pull through that one source, kept in [`Pulls`], and
//! answer not held when it fails; [`info`] is the image's
//! `/DIVERGE.md`, from the daemon's record or read through a source,
//! and nowhere else; [`name`] is the repository path the
//! daemon answers a has with, which the asking provider's own registry
//! serves the image as and never reads.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod info;
mod locate;
mod name;
mod pull;

pub use info::*;
pub use locate::*;
pub use name::*;
pub use pull::*;
