//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, finds the
//! acceptor, pairs the two halves of the connection, relays it both
//! ways, and finishes the scope. Its own files are flattened into it.

mod handle;

pub use handle::*;
