//! Performing a listing, rather than describing it.
//!
//! [`execute`] opens the scope and hands back an [`ExecuteStream`] of
//! what the provider sends — every image the caller built as added,
//! the word that the listing is whole, then images added and removed
//! as the caller builds and deletes them — with the [`Stop`] that
//! ends it. A listing has no end of its own, so the stop is how a
//! caller that has read what it wanted lets the scope go.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_stream;
mod stop;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
pub use stop::*;
