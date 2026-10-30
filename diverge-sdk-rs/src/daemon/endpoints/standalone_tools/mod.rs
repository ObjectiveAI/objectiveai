//! Standalone tools: the MCP servers the daemon runs for its agents
//! on their own record, and names.
//!
//! A tool is a tool container — an MCP server in a container, the
//! provider protocol's `containers::tools::run` — that the daemon holds
//! under a name of the caller's choosing. It comes to be one of two
//! ways: a [`create`] makes it from a [`template`](templates) — the
//! image, the limits and the arguments, held by its hash
//! — with the provider it is pinned to and the mounts that are its own,
//! exactly as an [`agent`](super::agents) is, and the daemon runs it;
//! or a [`register`] names a tool another daemon holds, by a record of
//! that daemon and as that daemon names it, and the daemon reaches it
//! over its connection to that daemon — that daemon's [`connect`],
//! which carries the tool's MCP exchanges and nothing else — and never
//! runs it. What a tool is FOR is being attached to agents: an
//! [`attach_agent_instance`] puts it among the MCP servers the daemon
//! answers one agent's tool calls with, and an
//! [`attach_agent_template`] among every agent's of a template —
//! those made already and those made after — under the tool's name,
//! and the agent's merged tool list says which tool serves each entry
//! under `_meta`, its image and its key, as
//! [`shared::mcp`](crate::shared::mcp) states. A tool may be attached
//! to any number of agents and templates at once; an agent attached
//! both ways has it once; a [`detach_agent_instance`] or a
//! [`detach_agent_template`] takes it back the way it was put on.
//! Either lands on an active agent as on an idle one: the agent's
//! proxy serves the tool, or serves it no more, from that instant,
//! and tells the agent its tool list changed; a call in flight is
//! answered still.
//!
//! # One container per tool
//!
//! However many agents a tool is attached to, one tool container runs,
//! and what keeps it running is its template's `mode`. A transient
//! tool's container runs while it is used — an agent's call in
//! flight, a file operation, a connect scope open on it — and is
//! stopped `idle_seconds` after the last use; an attachment alone
//! keeps nothing running, and an agent's loop begins by starting the
//! tools it is served. A service's container is kept running while
//! the tool is attached to any agent or any template, or any connect
//! is open on it, and retired `idle_seconds` after the last of those
//! leaves. A tool attached nowhere and connected to by nobody runs
//! nowhere. A create runs nothing. A connected tool's connection is
//! held as a transient tool's container is.
//!
//! [`templates`] holds what tools are made from; [`create`] makes a
//! tool under a name from one; [`edit`] changes what a created one
//! mounts; [`register`] holds another daemon's under a name;
//! [`attach_agent_instance`] and [`detach_agent_instance`] put it on
//! one agent and take it off, [`attach_agent_template`] and
//! [`detach_agent_template`] on every agent of a template; [`delete`]
//! removes a tool that is attached nowhere;
//! [`get`] answers one as a list would; [`list`] lists them, narrowed,
//! with the agents and the templates each is attached to and its tags; [`tag`] and
//! [`untag`] change a tool's tags; [`connect`] serves a created tool
//! to another daemon, and holds the tool while it is open;
//! [`filetree`] watches one's container whole; [`download`] sends the
//! client a file or a directory out of a tool's container, [`upload`]
//! puts files into it, and [`transfer`] copies out of it into an agent,
//! another tool or a volume, the bytes never reaching the client — the
//! four of a created tool only, since a connected tool's files are on
//! the other daemon. [`logs`] reads what the daemon kept of every call
//! and read it routed into a tool. A dependency an agent declares at register time
//! is no tool of this family: the daemon deploys it then and there,
//! holds its template as a record of the
//! [`dependency_tools`](super::dependency_tools) family, and serves the
//! running tool there, under its template; no request here finds
//! one. A tool declares no dependencies.

pub mod attach_agent_instance;
pub mod attach_agent_template;
pub mod connect;
pub mod create;
pub mod delete;
pub mod detach_agent_instance;
pub mod detach_agent_template;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod logs;
pub mod register;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;
