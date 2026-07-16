//! Consumer + wire types for the cli daemon's
//! `/agents/instances/{*aih}` endpoint — one agent's full
//! conversation, history + live.
//!
//! On connect the daemon replays the agent's conversation from the DB
//! as [`AgentInstanceEvent::Row`] events (each a keyed FULL-VALUE
//! [`ConversationRow`] — never a delta), sends
//! [`AgentInstanceEvent::Live`], then streams rows as they occur,
//! teed straight from the CLI's log writer (not gated on the DB
//! insert). A re-sent row identity replaces the prior value, which is
//! the whole merge story and also converges the snapshot/live seam.
//!
//! The daemon side lives in `objectiveai-cli`'s
//! `http::agent_instance_route` (fed by the log writer's
//! `db::logs::tee` over `conversation.sock`).

mod wire;
pub use wire::*;
mod listener;
pub use listener::*;
