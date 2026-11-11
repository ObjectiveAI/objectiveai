//! Performing a message, rather than describing it.
//!
//! [`execute`] opens the scope and hands back the [`Sending`]: the one
//! answer to wait on, and the cancel.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod cancel;
mod error;
mod execute;
mod sending;

pub use cancel::*;
pub use error::*;
pub use execute::*;
pub use sending::*;
