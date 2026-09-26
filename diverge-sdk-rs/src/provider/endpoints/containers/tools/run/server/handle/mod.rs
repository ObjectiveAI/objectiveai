//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, brings the
//! container up in order, serves it for its life, and finishes the
//! scope. Its own files are flattened into it.

mod family;
mod handle;

pub(crate) use family::*;
pub use handle::*;
