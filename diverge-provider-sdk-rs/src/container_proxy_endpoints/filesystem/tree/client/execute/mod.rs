//! Performing a tree, rather than describing it.
//!
//! [`execute`] opens the scope naming what to leave out and hands
//! back an [`ExecuteStream`] of the tree — a snapshot, then every
//! change, for as long as the scope lives — and an [`ExecuteHandle`]
//! whose one method sends the channel request that stops it. Ending
//! a tree is something the server says rather than something that
//! happens when a value goes out of scope: a destructor could not
//! await it, could not report it failing, and could not be told to
//! leave the tree running.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_handle;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
