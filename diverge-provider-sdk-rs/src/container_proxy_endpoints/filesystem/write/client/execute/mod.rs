//! Performing a write, rather than describing it.
//!
//! [`execute`] opens the scope naming the file, answers the one
//! channel the proxy opens with the content, piece by piece, and
//! waits for the proxy to say the file landed. One call is the whole
//! write.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
