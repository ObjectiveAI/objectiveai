//! Asking an agent something: a chat message, or a structured
//! decision.
//!
//! Two requests, one for each thing an agent's image may say it does
//! in its `/DIVERGE.md`:
//!
//! - [`chat`] hands the agent a message — MCP content blocks — which
//!   joins its conversation.
//! - [`structured_decisions`] hands it a prompt and typed questions,
//!   which it answers with numbers.
//!
//! Both are one question and one answer, open until the fate is
//! known, and both go to the agent in the order the daemon admitted
//! them, whichever kind each is. They are handed over differently: a
//! chat message joins a loop that is running, and a structured
//! decision waits for the agent to be idle, runs alone, and is
//! finished before anything after it starts — see
//! [`enqueue`](crate::shared::containers::enqueue).
//!
//! One grant covers both: `message`, over agents.

pub mod chat;
pub mod structured_decisions;
