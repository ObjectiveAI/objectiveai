//! Performing a write, rather than describing it.
//!
//! [`execute`] opens the scope naming the volume and the file, answers
//! the one channel the provider opens with the content, piece by
//! piece, and waits for the provider to say the file landed. One call
//! is the whole write.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
