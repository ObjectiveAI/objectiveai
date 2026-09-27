//! The list response: the tools, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Tool`], or a
//! failure. [`Tool`] is one tool as the daemon holds it: its name,
//! where it comes from — its [`Origin`], created from an image or
//! connected to somebody else's container — whether its container
//! runs, and the agents it is attached to.

mod frame;
mod origin;
mod tool;

pub use frame::*;
pub use origin::*;
pub use tool::*;
