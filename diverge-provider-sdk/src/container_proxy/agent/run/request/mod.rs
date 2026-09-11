//! The ask: the prompt, as
//! [`shared::containers::run_loop::request`](crate::shared::containers::run_loop::request),
//! re-exported. The agent is not here: it was registered, once, on
//! [`/agent/register`](super::super::register).

pub use crate::shared::containers::run_loop::request::*;
