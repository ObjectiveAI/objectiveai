//! Connected standalone tools: another daemon's standalone tools, held
//! here on their own record, and names, for this daemon's agents.
//!
//! A connected tool is a standalone tool another daemon holds and runs
//! — one of that daemon's
//! [`standalone_tools`](super::standalone_tools) — that this daemon
//! holds under a name of the caller's choosing. A [`create`] names it
//! by a record of that daemon, a route beyond it, and the tool as that
//! daemon names it, and the daemon reaches it over its connection to
//! that daemon — that daemon's
//! [`standalone_tools::connect`](super::standalone_tools::connect),
//! which carries the tool's MCP exchanges and nothing else — and never
//! runs it, starts it or stops it. Nothing of what the tool is made
//! from is known here: its image, its limits, its mounts and its mode
//! are the other daemon's, and so are its files, its commands, its
//! terminal and its frontend, which no request here reaches.
//!
//! What a connected tool is FOR is being attached to agents, exactly as
//! a standalone tool is: an [`attach_agent_instance`] puts it among the
//! MCP servers the daemon answers one agent's tool calls with, and an
//! [`attach_agent_template`] among every agent's of a template — those
//! made already and those made after — under the tool's name, and the
//! agent's merged tool list says which tool serves each entry under
//! `_meta`, as [`shared::mcp`](crate::shared::mcp) states: the kind
//! `connected`, with the image and the template the other daemon's
//! connect told. A tool may be attached to any number of agents and
//! templates at once; an agent attached both ways has it once; a
//! [`detach_agent_instance`] or a [`detach_agent_template`] takes it
//! back the way it was put on. Either lands on an active agent as on
//! an idle one: the agent's proxy serves the tool, or serves it no
//! more, from that instant, and tells the agent its tool list changed;
//! a call in flight is answered still.
//!
//! # One connect per tool
//!
//! However many agents a connected tool is attached to, the daemon
//! holds at most one connect on it, as a transient standalone tool's
//! container is held: opened for the first use — an agent's call in
//! flight — and let go `idle_seconds` after the last. An attachment
//! alone holds nothing open, and a create connects nothing. Whether
//! the connect is held is the tool's `deployed` in a [`list`].
//!
//! [`create`] holds another daemon's tool under a name; [`edit`]
//! changes its name; [`attach_agent_instance`] and
//! [`detach_agent_instance`] put it on one agent and take it off,
//! [`attach_agent_template`] and [`detach_agent_template`] on every
//! agent of a template; [`delete`] removes one that is attached
//! nowhere; [`get`] answers one as a list would; [`list`] lists them,
//! narrowed, with the agents and the templates each is attached to and
//! its tags; [`tag`] and [`untag`] change a tool's tags.
//! [`logs`](super::logs) reads what the daemon kept of every call and
//! read it routed into one. Every request names the tool by a
//! [`reference::ConnectedStandaloneTool`](crate::daemon::reference::ConnectedStandaloneTool):
//! its name, or the daemon, the tool there and its index. No request
//! of the [`standalone_tools`](super::standalone_tools) family finds a
//! connected tool, and no request here finds a standalone one.

pub mod attach_agent_instance;
pub mod attach_agent_template;
pub mod create;
pub mod delete;
pub mod detach_agent_instance;
pub mod detach_agent_template;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
