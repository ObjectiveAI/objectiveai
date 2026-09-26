//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](crate::wire::server::session::Session) yielded, holds the
//! volume, and answers every ask until the stop.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
