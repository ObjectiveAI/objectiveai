//! The list response: the routes, one value each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a failure.
//! [`Route`] is what a value is without a program, and the reference
//! for what a program is run over: one route as the daemon holds it —
//! its path, the tool it routes to, when it was put down and by whom.

mod frame;
mod route;

pub use frame::*;
pub use route::*;
