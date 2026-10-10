//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the connection's identity, holds
//! the identity's accept slot, pairs every connection the connect
//! handlers open on it with the half the daemon opens back, and
//! finishes the scope. Its own files are flattened into it.

mod handle;

pub use handle::*;
