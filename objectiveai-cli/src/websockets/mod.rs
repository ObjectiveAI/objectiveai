//! Process-level WebSockets infrastructure shared between the
//! `agents spawn` and `functions execute` command leaves.
//!
//! Each agent-completion / function-execution call opens a single
//! upstream WebSocket connection to the API server and drives chunks
//! through it. The pieces below are the cross-cutting state every
//! such call needs:
//!
//! - [`agent_registry`] — process-owned `flock`-style claim files
//!   keyed by `agent_instance_hierarchy`. Mutual exclusion across
//!   processes for a given agent slot. Backed by the crate-root
//!   [`crate::lock_file`] primitive (objectiveai-db keeps a copy
//!   for its postgres bootstrap mutex).
//! - [`mcp_server`] — the in-process `objectiveai-mcp` server handle
//!   the conduit forwards plugin tool calls to.
//! - [`conduit`] — the MCP reverse-attach proxy that routes WS
//!   request frames out to upstream plugin MCPs.
//! - [`agent_hierarchies`] — recursive iterator trait that yields
//!   every `agent_instance_hierarchy` referenced by a chunk; used
//!   by the per-chunk claim hook.

pub mod agent_hierarchies;
pub mod agent_registry;
pub mod conduit;
pub mod mcp_server;
