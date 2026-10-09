//! Answering the request, rather than describing it.
//!
//! [`handle`] takes the scope a
//! [`Session`](crate::wire::server::session::Session) yielded, finds the
//! container, opens the proxy's serve of the subtree, and relays
//! every ask and every tree until the stop.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod handle;

pub use handle::*;
