//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](diverge_sdk::wire::server::session::Session) yielded,
//! posts the message, and answers its fate.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
