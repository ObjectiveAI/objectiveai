//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, asks the
//! runner whether the connector may attach, serves the connection, and finishes the
//! scope. Its own files are flattened into it.

mod family;
mod handle;

pub(crate) use family::*;
pub use handle::*;
