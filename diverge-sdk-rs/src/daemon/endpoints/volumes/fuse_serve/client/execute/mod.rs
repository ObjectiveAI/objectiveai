//! Performing a volume FUSE serve: the exchange, rather than the
//! description of it.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod error;
mod execute;

pub use error::*;
pub use execute::*;

/// The served scope, held: the provider scope's own handle, since a
/// daemon's FUSE serve scope and a provider's share their channels
/// byte for byte. See
/// [`volumes::fuse_serve::client::execute::ExecuteHandle`](crate::provider::endpoints::volumes::fuse_serve::client::execute::ExecuteHandle).
pub type ExecuteHandle = crate::provider::endpoints::volumes::fuse_serve::client::execute::ExecuteHandle;

/// How a served scope ended: the provider scope's own.
pub type End = crate::provider::endpoints::volumes::fuse_serve::client::execute::End;

/// An ask a served scope could not carry: the provider scope's own.
pub type AskError = crate::provider::endpoints::volumes::fuse_serve::client::execute::AskError;
