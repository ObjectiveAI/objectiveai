//! The agent: a Python source and what it needs installed, as the
//! caller states them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer. One member is not this image's to define:
//! `mcp_tools`, the tool containers the agent depends on, in the form
//! the provider's wire gives a tool, passed back whole as the
//! registration's answer.
//!
//! No model. The agent IS the source: every turn runs it against the
//! whole conversation, and what its last expression evaluates to is
//! the turn. Reworked from the provider SDK's reference Python agent
//! the way the Codex agent was, keeping what the harness honors and
//! dropping what nothing here reads:
//! - `upstream`: the image is the discriminator; a value that reaches
//!   this container is a Python agent by arrival.
//! - `memory` and `disk`: ceilings are the container request's
//!   business, set by whoever runs the container, not the agent's
//!   vocabulary. (A tool the agent asks for under `mcp_tools` names
//!   its own, which are that tool container's.)
//! - Any in-process reach back into the container — the CLI's
//!   `objectiveai.execute`: the script calls tools only through the
//!   `assistant_tool_call` chunks it emits, and reads the answers on
//!   its next run.

mod agent;
mod operator;
mod version;

pub use agent::*;
pub use operator::*;
pub use version::*;
