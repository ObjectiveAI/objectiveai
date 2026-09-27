//! The list response: the tools, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Tool`], or a
//! failure. [`Tool`] is one tool as the daemon holds it: its name,
//! its image, whether its container runs, its provider, and the
//! agents it is attached to.

mod frame;
mod tool;

pub use frame::*;
pub use tool::*;
