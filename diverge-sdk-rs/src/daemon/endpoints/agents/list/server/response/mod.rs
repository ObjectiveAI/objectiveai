//! The list response: the agents, one value each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a
//! failure. [`Agent`] is what a value is without a program, and the
//! reference for what a program is run over: one agent as the
//! daemon holds it — its name, its template and its number among
//! the agents made from it, who made it — its
//! [`creator`](crate::daemon::creator) — its activity, its
//! provider, its log's length, the tools attached to it, and its
//! tags.

mod agent;
mod frame;

pub use agent::*;
pub use frame::*;
