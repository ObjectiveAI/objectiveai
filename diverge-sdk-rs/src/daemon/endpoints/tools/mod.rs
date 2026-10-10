//! Tools: the MCP servers the daemon runs for its agents, and names.
//!
//! A tool is a tool container — an MCP server in a container, the
//! provider protocol's `containers::tools::run` — that the daemon holds
//! under a name of the caller's choosing. It comes to be one of two
//! ways: a [`create`] makes it from a [`template`](templates) — the
//! image, the limits and the arguments, held by its hash
//! — with the provider it is pinned to and the mounts that are its own,
//! exactly as an [`agent`](super::agents) is, and the daemon runs it;
//! or a [`connect`] names a tool another daemon holds, by a record of
//! that daemon and as that daemon names it, and the daemon joins it —
//! through that daemon's [`expose`], and the provider protocol's
//! `containers::tools::connect` on the provider the expose names — and
//! never runs it. What a tool is FOR is being attached to agents: an
//! [`attach`] puts it among the MCP servers the daemon answers an
//! agent's tool calls with, under the tool's name, and the agent's
//! merged tool list says which tool serves each entry under `_meta`,
//! its image and its key, as [`shared::mcp`](crate::shared::mcp)
//! states. A tool may be attached to any number of agents at once, and
//! a [`detach`] takes it back from one.
//!
//! # One container per tool
//!
//! However many agents a tool is attached to, one tool container runs:
//! the daemon starts it when an attached agent becomes active and no
//! attached agent is, serves every attached agent's calls to it, and
//! stops it when no attached agent is active and no expose holds it. A
//! tool attached nowhere and exposed to nobody runs nowhere. A create
//! runs nothing.
//!
//! [`templates`] holds what tools are made from; [`create`] makes a
//! tool under a name from one; [`edit`] changes what a created one
//! mounts; [`connect`] holds another daemon's under a name; [`attach`]
//! and [`detach`] put it on an agent and take it off, the attach
//! allowed while the agent is active and the detach only while it is
//! not; [`delete`] removes a tool that is attached nowhere; [`get`]
//! answers one as a list would; [`list`] lists them, narrowed, with the
//! agents each is attached to and its tags; [`tag`] and [`untag`]
//! change a tool's tags; [`expose`] hands another daemon what joins a
//! created tool's container, and holds the tool while it is open;
//! [`filetree`] watches one's container whole; [`download`] sends the
//! client a file or a directory out of a tool's container, [`upload`]
//! puts files into it, and [`transfer`] copies out of it into an agent,
//! another tool or a volume, the bytes never reaching the client. A
//! dependency an agent declares at register time is deployed by the
//! daemon then and there, as a tool of the agent's own that lives
//! while the agent's container does: listed among the tools as
//! `dependency` kind, reached by its agent and its template, read and
//! never changed. A tool declares no dependencies.

pub mod attach;
pub mod connect;
pub mod create;
pub mod delete;
pub mod detach;
pub mod download;
pub mod edit;
pub mod expose;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;
