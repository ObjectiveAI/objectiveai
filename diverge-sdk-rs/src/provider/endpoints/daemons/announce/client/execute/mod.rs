//! Performing an announce, rather than describing it.
//!
//! [`execute`] opens the scope and hands back an [`ExecuteStream`]
//! that ends when the scope does, with the [`Cancel`] that ends it. An
//! announce has no end of its own, so the cancel is how a caller
//! leaves the listing.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cancel;
mod error;
mod execute;
mod execute_stream;

pub use cancel::*;
pub use error::*;
pub use execute::*;
pub use execute_stream::*;
