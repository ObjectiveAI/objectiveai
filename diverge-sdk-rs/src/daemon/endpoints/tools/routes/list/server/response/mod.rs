//! The list response: a route added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Route`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Route`] is what one is: one route as the daemon
//! holds it — its path, the tool it routes to, when it was put down and
//! by whom.

mod frame;
mod route;

pub use frame::*;
pub use route::*;
