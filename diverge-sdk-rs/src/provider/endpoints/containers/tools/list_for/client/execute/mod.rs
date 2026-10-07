//! Performing a listing, rather than describing it.
//!
//! [`execute`] opens the scope naming the identity whose containers
//! are asked about, and hands back an [`ExecuteStream`] of the
//! containers the provider sends, each as its runner says yes, to
//! the finish — or the provider's error, last. A listing ends by
//! itself, so there is no handle.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
