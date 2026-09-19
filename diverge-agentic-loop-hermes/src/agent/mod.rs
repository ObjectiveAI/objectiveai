//! The agent: Hermes's parameters, as the caller states them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer. One member is not this image's to define:
//! `mcp_tools`, the tool containers the agent depends on, in the form
//! the provider's wire gives a tool, passed back whole as the
//! registration's answer.

pub mod provider;
pub mod toolsets;

mod agent;
mod effort;

pub use agent::*;
pub use effort::*;
pub use provider::Provider;
pub use toolsets::Toolsets;
