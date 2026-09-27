//! Tools: the MCP servers the daemon runs for its agents, and names.
//!
//! A tool is a tool container — an MCP server in a container, the
//! provider protocol's `containers::tools::run` — that the daemon
//! holds under a name of the caller's choosing, made from what its
//! [`create`] names: the image, the limits, the mounts, the
//! arguments, and the provider it is pinned to, if any, exactly as an
//! [`agent`](super::agents) is. What a tool is FOR is being attached
//! to agents: an [`attach`] puts it among the MCP servers the daemon
//! answers an agent's tool calls with, under the tool's name, and the
//! agent's merged tool list says which image serves each tool under
//! `_meta`, as the provider protocol provides. A tool may be attached
//! to any number of agents at once, and a [`detach`] takes it back
//! from one.
//!
//! # One container per tool
//!
//! However many agents a tool is attached to, one tool container
//! runs: the daemon starts it when an attached agent becomes active
//! and no attached agent is, serves every attached agent's calls to
//! it, and stops it when no attached agent is active. A tool attached
//! nowhere runs nowhere. A create runs nothing.
//!
//! [`create`] makes a tool under a name; [`attach`] and [`detach`]
//! put it on an agent and take it off, the attach allowed while the
//! agent is active and the detach only while it is not; [`delete`]
//! removes a tool that is attached nowhere; [`list`] names every one
//! the caller has, with the agents each is attached to.

pub mod attach;
pub mod create;
pub mod delete;
pub mod detach;
pub mod list;
