//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, asks the
//! runner of every tool container the identity has whether the
//! lister may see it,
//! sends each container as its runner allows, and finishes the scope.

mod handle;

pub use handle::*;
