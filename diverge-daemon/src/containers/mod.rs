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
//! [`Inflight`] — and [`idle_tool`] a connected tool's, running only
//! while nothing uses it; [`message`] is the one way into an agent;
//! [`use_tool`] and [`release`] are "one container per tool", started
//! for the first container or connect scope that uses it and stopped
//! with the last, a [`User`] either way;
//! [`mcp`] is the one tool list an agent sees and the routing of its
//! calls; [`deploy`] deploys a container's declared dependencies then
//! and there, each a tool of the agent's own for the agent's life;
//! [`fuse`] serves the mounts — another provider's volume bridged, or
//! an agent's path into its dependency. [`Key`] names a container by
//! its record or, a dependency, by its deployment, [`ToolKey`] a
//! tool's run either way; [`AgentRun`] and [`ToolRun`] are what is
//! live for one; [`container`] and [`dependency`] are the request a
//! run is; [`Opened`] is a run as a file operation works on it, and
//! [`files`] the operations — the container's tree whole, a file
//! read, a file written — with the daemon's mounts included.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod build;
mod caller;
mod error;
mod idle;
mod inflight;
mod key;
mod opened;
mod provider;
mod pump;
mod run;
mod start;
mod stop;
mod tools;

pub mod files;

pub mod answerers;
pub mod deploy;
pub mod fuse;
pub mod mcp;
pub mod message;

pub use build::*;
pub use caller::*;
pub use error::*;
pub use idle::*;
pub use inflight::*;
pub use key::*;
pub use opened::*;
pub use provider::*;
pub use pump::*;
pub use run::*;
pub use start::*;
pub use stop::*;
pub use tools::*;
