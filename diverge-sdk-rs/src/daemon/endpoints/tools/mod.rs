//! Tools: the MCP servers the daemon runs for its agents, and names.
//!
//! A tool is a tool container — an MCP server in a container, the
//! provider protocol's `containers::tools::run` — that the daemon
//! holds under a name of the caller's choosing. It comes to be one of
//! two ways: a [`create`] makes it from a [`template`](templates) —
//! the image, the limits, the resources and the arguments, held by
//! its hash — with the provider it is pinned to and the mounts that
//! are its own, exactly as an [`agent`](super::agents) is, and the
//! daemon runs it; or a [`connect`] names a container somebody
//! else runs, by its provider, its id and an authorization, and the
//! daemon joins it with the provider protocol's
//! `containers::tools::connect` and never runs it. Whom a daemon
//! admits as a connector to a tool it runs is that daemon's
//! configuration, stated on no request. What a tool is FOR is being attached
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
//! [`templates`] holds what tools are made from; [`create`] makes a
//! tool under a name from one; [`edit`] changes what a
//! created one mounts; [`connect`] holds somebody else's under a
//! name; [`attach`] and [`detach`]
//! put it on an agent and take it off, the attach allowed while the
//! agent is active and the detach only while it is not; [`delete`]
//! removes a tool that is attached nowhere; [`get`] answers one as
//! a list would; [`list`] lists them,
//! narrowed, with the agents each is attached to and its tags;
//! [`tag`] and [`untag`] change a tool's tags.

pub mod attach;
pub mod connect;
pub mod create;
pub mod delete;
pub mod detach;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod templates;
pub mod untag;
