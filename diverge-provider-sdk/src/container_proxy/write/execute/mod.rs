//! Writing a file in, rather than describing the write.
//!
//! [`execute`] opens `/write`, names the file, streams its content,
//! and reports whether it landed. Its own files are flattened into
//! it.

mod error;
mod execute;

pub use error::*;
pub use execute::*;
