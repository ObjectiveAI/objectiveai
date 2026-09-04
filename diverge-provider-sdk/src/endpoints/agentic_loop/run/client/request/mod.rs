//! Agentic loop request data.
//!
//! What a caller hands a provider to start or resume a loop: an
//! [`Agent`](agent::Agent) to run and a prompt to run it on.
//!
//! There is no conversation here, and no continuation either.
//! Everything before this turn lives in the continuation — the
//! provider's own state, which the server FETCHES from the caller as
//! the run starts rather than reading out of the request — so a
//! caller sends what is new rather than replaying a history it would
//! otherwise have to keep in step.
//!
//! Everything here is POST-TRANSFORM. See [`agent`] for what that
//! excludes and why.

pub mod agent;

mod frame;
mod mount;

pub use frame::*;
pub use mount::*;
