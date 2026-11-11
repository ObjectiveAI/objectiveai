//! Performing a listing, rather than describing it.
//!
//! [`execute`] opens the scope and hands back an [`ExecuteStream`] of
//! what the provider sends — every image in the caller's Store as
//! added, the word that the listing is whole, then images added,
//! changed and removed as the caller builds, pushes, edits and deletes
//! them — with the [`Cancel`] that
//! ends it. A listing has no end of its own, so the cancel is how a
//! caller that has read what it wanted lets the scope go.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_stream;
mod cancel;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
pub use cancel::*;
