//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope and the decoded request, asks the
//! runner of every tool container the identity has whether the
//! lister may see it, sends each container as its runner allows,
//! says when the listing is whole, sends every container the
//! identity starts and ends after, and finishes the scope at the
//! lister's stop.

mod handle;

pub use handle::*;
