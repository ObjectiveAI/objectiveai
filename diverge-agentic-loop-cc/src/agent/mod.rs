//! The agent: Claude Code's parameters, as the caller states them.
//!
//! This is what the `agent` value on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer.

mod agent;
mod effort;
mod tools;

pub use agent::*;
pub use effort::*;
pub use tools::*;
