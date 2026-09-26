//! Performing a read, rather than describing it.
//!
//! [`execute`] opens the scope naming the file and hands back an
//! [`ExecuteStream`] of its bytes, piece by piece, to the finish — or
//! the proxy's error, last. A read ends by itself, so there is no
//! handle.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
