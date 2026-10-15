//! Performing an image deletion, rather than describing it.
//!
//! [`execute`] asks and reads the one answer.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;

pub use execute::*;
