//! Consumer + wire types for the cli daemon's `/agents` endpoint — a
//! live stream of every agent's active/inactive status.
//!
//! On connect the daemon sends one [`AgentEvent::Snapshot`] with every
//! agent (from the DB), each carrying spawn/last-active timestamps and a
//! current `active` flag, then streams [`AgentEvent::Activated`] /
//! [`AgentEvent::Deactivated`] deltas as agents acquire / release their
//! per-instance lock. Because "inactive" is driven by the OS releasing
//! the agent's lockfile (guaranteed even on process kill), a spawn killed
//! mid-stream is reported inactive exactly, with no leak.
//!
//! The daemon side lives in `objectiveai-cli`'s
//! `websockets::websocket_agents`.

mod wire;
pub use wire::*;
mod listener;
pub use listener::*;
