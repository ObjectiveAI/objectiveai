//! The work layer: containers running on providers, and everything
//! that runs through them.
//!
//! An agent or a tool is its record; the container is work made from
//! it when it is first used and ended when it has gone unused for
//! `idle_seconds`, or stopped with the daemon. [`agent`] and [`tool`]
//! make the run — the providers it may run on, [`candidates`], tried
//! in order; the mounts served; the [`answerers`] that take the
//! provider's asks on the run scope; the `run` opened — and
//! [`stop_agent`], [`ended_agent`], [`ended_tool`] and [`stop_all`]
//! end it; [`pump`] reads the agent's conversation off the run into
//! its log and keeps whether a loop runs; [`idle`] is the clock,
//! running only while the agent is not active — no loop, and nothing
//! [`Inflight`] — and [`idle_tool`] a transient tool's, running only
//! while nothing holds it; [`message`] is the one way into an agent;
//! [`use_tool`] is "one container per tool", started for the first
//! thing that uses it and held up by what is in flight into it and
//! the connect scopes on it — [`connect`] and [`disconnect`] — a
//! transient one stopped by its clock `idle_seconds` after the last
//! use; [`attached`] and [`detached`] keep running agents' served
//! sets in step with the attachments; [`running_dependency`] is
//! the dependency tool a reference names, found running or not at
//! all, since one is started by nobody's request;
//! [`mcp`] is the one tool list an agent sees and the routing of its
//! calls; [`deploy`] records a container's declared dependencies as
//! dependency tools under their owners and deploys them then and there
//! — a transient tool of the agent's own for the agent's run, a
//! service kept by [`services`] for as long as any agent is attached
//! to it and stopped after, as a tool on record whose template's mode
//! is a service is kept while any agent is attached or any connect
//! is open;
//! [`fuse`] serves the mounts — another provider's volume bridged, or
//! an agent's path into its dependency; [`images`] answers a
//! provider's image asks from the other providers connected. [`Key`]
//! names a container by
//! its record or, a dependency tool, by its row, [`ToolKey`] a
//! tool's run either way; [`AgentRun`] and [`ToolRun`] are what is
//! live for one; [`container`] and [`dependency`] are the request a
//! run is, and [`secrets`] what a run's secrets come to — the names
//! its program takes bound to vault keys, and the slots held while
//! it is active; [`activate_agent`] and [`activate_tool`] ready a
//! run for work, its values taken and fed, [`inactive_agent`] and
//! [`inactive_tool`] write back what it made of them, and
//! [`pump_tool`] reads a tool container's activity to its end;
//! [`events`] writes the graph of who sent what to whom, at the
//! seams the exchanges pass through;
//! [`Opened`] is a run as a file operation works on it, and
//! [`files`] the operations — the container's tree whole, a file
//! read, a file written — with the daemon's mounts included.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod activate;
mod attached;
mod build;
mod error;
pub mod capable;
mod idle;
mod inflight;
mod key;
mod opened;
mod provider;
mod pump;
mod pump_tool;
mod run;
mod running;
mod start;
mod stop;
mod tools;

pub mod files;
pub mod secrets;

pub mod answerers;
pub mod deploy;
pub mod events;
pub mod fuse;
pub mod images;
pub mod mcp;
pub mod message;
pub mod permissions;
pub mod services;
pub mod tool_log;

pub use activate::*;
pub use attached::*;
pub use build::*;
pub use error::*;
pub use idle::*;
pub use inflight::*;
pub use key::*;
pub use opened::*;
pub use provider::*;
pub use pump::*;
pub use pump_tool::*;
pub use run::*;
pub use running::*;
pub use start::*;
pub use stop::*;
pub use tools::*;
