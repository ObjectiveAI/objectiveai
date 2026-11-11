//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](diverge_sdk::wire::server::session::Session) yielded and
//! keeps the caller in the provider's
//! [`Listables`](crate::protocol::listables::Listables), with the
//! request's announcement, until the caller cancels.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
