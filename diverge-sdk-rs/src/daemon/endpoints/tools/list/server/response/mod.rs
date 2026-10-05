//! The list response: the tools, one value each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a
//! failure. [`Tool`] is what a value is without a program, and the
//! reference for what a program is run over: one tool as the daemon
//! holds it — its name, where it comes from — its [`Origin`], created
//! from a template or connected to somebody else's container — and
//! its number among the tools made with the same, who made it — its
//! [`creator`](crate::daemon::creator) —
//! whether its container runs, the agents it is attached to, and its
//! tags.

mod frame;
mod origin;
mod tool;

pub use frame::*;
pub use origin::*;
pub use tool::*;
