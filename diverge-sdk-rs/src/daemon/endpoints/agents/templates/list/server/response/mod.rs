//! The list response: the templates, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`], or a
//! failure. [`Listed`] is one template as the daemon holds it: its
//! id, when it was made, and the template itself.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
