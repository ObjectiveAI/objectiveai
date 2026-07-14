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
//!   streams into, fanned out to every client of the WebSocket
//!   server's `/listen` route.
//! - [`daemon_execute`] — the daemon's `/execute` WebSocket route:
//!   connection-per-command in-process execution for remote consumers
//!   (the SDK's `SseCommandExecutor`, notably the viewer).
//! - [`websocket_agents`] — the daemon's `/agents/instances/list` WebSocket route + its
//!   dedicated `agents.sock` producer socket: a live all-agents
//!   active/inactive stream, driven by AIH-lockfile release.
//! - [`websocket_agent_instance`] — the daemon's
//!   `/agents/instances/{*aih}` WebSocket route + its dedicated
//!   `conversation.sock` producer socket: one agent's full
//!   conversation, DB history + live rows teed from the log writer.
//! - [`websocket_laboratory`] — the daemon's `/laboratory` WebSocket
//!   route (laboratory managers dial in, identify, then authorize) +
//!   the `laboratories.sock` request/response socket the conduit uses
//!   to reach connected laboratories.
//! - [`websocket_laboratories`] — the daemon's `/laboratories/list` +
//!   `/laboratories/{id}` SSE routes: the live laboratories
//!   merge (connected ∪ local scan) and per-laboratory records with
//!   attachments.

pub mod agent_hierarchies;
pub mod agent_registry;
pub mod conduit;
pub mod daemon_auth;
pub mod daemon_execute;
pub mod daemon_stream;
pub mod mcp_listener;
pub mod mcp_server;
pub mod websocket_agent_instance;
pub mod websocket_agents;
pub mod websocket_laboratories;
pub mod websocket_laboratory;
