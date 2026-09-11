//! Performing the exchange, rather than describing it.
//!
//! [`execute`] asks, and hands back the version.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;

pub use execute::*;
