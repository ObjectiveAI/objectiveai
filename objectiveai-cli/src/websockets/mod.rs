//! Process-level WebSockets infrastructure shared between the
//! `agents spawn` and `functions execute` command leaves.
//!
//! Each agent-completion / function-execution call opens a single
//! upstream WebSocket connection to the API server and drives chunks
//! through it. The pieces below are the cross-cutting state every
//! such call needs:
//!
//! - [`agent_registry`] — process-owned lock claims keyed by
//!   `agent_instance_hierarchy`. Mutual exclusion across processes
//!   for a given agent slot. Backed by [`objectiveai_sdk::lockfile`]
//!   at the per-agent layout in [`crate::command::agents::locks`].
//! - [`mcp_server`] — the in-process `objectiveai-mcp` server handle
//!   the conduit forwards plugin tool calls to.
//! - [`conduit`] — the MCP reverse-attach proxy that routes WS
//!   request frames out to upstream plugin MCPs.
//! - [`agent_hierarchies`] — recursive iterator trait that yields
//!   every `agent_instance_hierarchy` referenced by a chunk; used
//!   by the per-chunk claim hook.
//! - [`mcp_listener`] — per-`response_id` local-socket MCP endpoint
//!   that forwards ops to the API over the chunk-stream WS; spawned
//!   the first time a chunk surfaces a given agent-completion id.
//! - [`daemon_stream`] — the resident daemon's broadcast hub: a
//!   fixed-name local socket that producers feed CLI request/response
//!   streams into, fanned out to every client of the root WebSocket
//!   server.

pub mod agent_hierarchies;
pub mod agent_registry;
pub mod conduit;
pub mod daemon_auth;
pub mod daemon_stream;
pub mod mcp_listener;
pub mod mcp_server;
