//! Performing the exchange, rather than describing it.
//!
//! [`execute`] opens an accept and hands back an [`Accepting`]: the
//! identity the provider knows the daemon by, and the scope held for
//! as long as the daemon accepts, every connection the provider
//! announces on it answered through the daemon's
//! [`Acceptor`](crate::provider::client::Acceptor).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod accepting;
mod execute;

pub use accepting::*;
pub use execute::*;
