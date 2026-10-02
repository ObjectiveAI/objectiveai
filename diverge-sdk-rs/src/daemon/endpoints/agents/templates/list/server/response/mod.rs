//! The list response: the templates, one value each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a
//! failure. [`Listed`] is what a value is without a program, and the
//! reference for what a program is run over: one template as the
//! daemon holds it — its id, when it was made, its tags, and the
//! template itself.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
