//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](crate::server::session::Session) yielded, runs a
//! laboratory in a container, and serves everyone it concerns until it
//! is stopped.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
