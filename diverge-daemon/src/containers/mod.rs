//! The work layer: containers running on providers, and everything
//! that runs through them.
//!
//! An agent or a tool is its record; the container is work made from
//! it when it is first used and ended when it has gone unused for
//! `idle_seconds`, or stopped with the daemon. [`agent`] and [`tool`]
//! make the run — the provider chosen, the mounts served, the
//! [`answerers`] that take the provider's asks on the run scope, the
//! `run` opened — and [`stop_agent`], [`ended_agent`], [`ended_tool`]
//! and [`stop_all`] end it; [`pump`] reads the agent's conversation
//! off the run into its log and keeps whether a loop runs; [`idle`]
//! is the clock; [`message`] is the one way into an agent;
//! [`use_tool`] and [`release`] are "one container per tool", started
//! for the first container that uses it and stopped with the last;
//! [`mcp`] is the one tool list an agent sees and the routing of its
//! calls; [`deploy`] answers a container's declared dependencies — by
//! a route, by an attachment, or by the deployer agent's queue;
//! [`fuse`] serves the mounts, a resource's bytes from the daemon's
//! own content or another provider's volume bridged. [`Key`] names a
//! container by its record; [`AgentRun`] and [`ToolRun`] are what is
//! live for one; [`choose`] is which provider; [`container`] is the
//! request a run is; [`Opened`] is a run as a file operation works on
//! it, and [`files`] the operations — the container's tree whole, a
//! file read, a file written — with the daemon's mounts included.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod build;
mod caller;
mod error;
mod idle;
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
pub use key::*;
pub use opened::*;
pub use provider::*;
pub use pump::*;
pub use run::*;
pub use start::*;
pub use stop::*;
pub use tools::*;
