//! The list response: the resources, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`], or a
//! failure. [`Listed`] is one resource as the daemon holds it: its
//! id, its kind, its description, when it was uploaded, and its
//! size.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
