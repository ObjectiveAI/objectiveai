//! The list response: the agents, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Agent`], or a
//! failure. [`Agent`] is one agent as the daemon holds it: its name,
//! its image, its activity, its provider, its log's length, and what
//! waits for it.

mod agent;
mod frame;

pub use agent::*;
pub use frame::*;
