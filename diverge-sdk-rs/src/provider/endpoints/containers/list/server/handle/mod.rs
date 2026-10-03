//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, asks every
//! runner the identity has whether the lister may see its container,
//! sends each container as its runner allows, and finishes the scope.

mod handle;

pub use handle::*;
