//! Agentic loop request data.
//!
//! What a caller hands a provider to start or resume a loop: an
//! [`Agent`] to run and a prompt to run it on.
//!
//! There is no conversation here. Everything before this turn lives in
//! the continuation, which is the provider's own state — so a caller
//! sends what is new rather than replaying a history it would
//! otherwise have to keep in step.
//!
//! Everything here is POST-TRANSFORM. See [`agent`] for what that
//! excludes and why.

pub mod agent;

mod request;

pub use agent::Agent;
pub use request::*;
