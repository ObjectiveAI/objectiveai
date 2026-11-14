//! Performing the daemon's read scope, rather than describing it.
//!
//! [`execute`] opens the scope of one image, reads whether the daemon
//! can supply it, and where it can hands back an [`ExecuteHandle`]: the
//! provider scope's own, since the channels are the same — one method
//! per ask, and the cancel.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod error;
mod execute;

pub use error::*;
pub use execute::*;

/// The held scope: the provider read scope's handle, whose channels this
/// scope shares byte for byte. See
/// [`images::read::client::execute::ExecuteHandle`](crate::provider::endpoints::images::read::client::execute::ExecuteHandle).
pub type ExecuteHandle = crate::provider::endpoints::images::read::client::execute::ExecuteHandle;

/// How a held scope ended: the provider scope's own.
pub type End = crate::provider::endpoints::images::read::client::execute::End;
