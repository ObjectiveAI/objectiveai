//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](diverge_sdk::wire::server::session::Session) yielded, the
//! caller's identity and the provider's
//! [`CapacityReporter`](crate::protocol::capacity_reporter::CapacityReporter),
//! and answers with what the caller may still use.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
