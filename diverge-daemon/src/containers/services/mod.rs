//! Services: the dependency tools the daemon keeps.
//!
//! A service is a dependency tool template an agent's program
//! declared as one, made under its scope key — the agent, the agent's
//! template, or the template alone — and kept by the daemon from then
//! on. What holds it up is its ATTACHMENTS, not any run: an agent is
//! attached by a registration that declares the template, detached by
//! one that no longer does, and detached by its delete; while any
//! agent is attached the service's container is kept running —
//! started at the registration that made it, started again whenever
//! its run ends, and started at every boot of the daemon's — and once
//! none is, its idle clock runs, and `idle_seconds` after the last
//! attachment left with none since it is RETIRED: its run stopped,
//! its row gone, for good. Its database scope is not retired with
//! it: that goes with its scope's owner — the agent, the agent
//! template, or the last agent template whose agents ever attached a
//! global service — at that owner's delete.
//!
//! [`register`] is what a registration does: every service declared
//! found or made, the agent attached, the rest detached; [`settle`]
//! what a start does when the program declared nothing, the agent
//! detached from every service; [`keep`] is a service kept up — its
//! [`State`] made, its [`supervise`] task and its [`clock`] started —
//! and [`held`] its hold recomputed from its attachments; [`start`]
//! is one run of its container; [`retire`] is the end; [`boot`] is
//! every service brought up at the daemon's start, every unattached
//! one retired; [`stop_all`] is the daemon stopping, the supervisors
//! stilled before the runs are stopped so that none comes back; and
//! [`agent_deleted`] and [`template_deleted`] are what the deletes do
//! to the services the deleted thing held up or owned.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod boot;
mod clock;
mod deleted;
mod held;
mod keep;
mod register;
mod retire;
mod settle;
mod start;
mod state;
mod stop_all;
mod supervise;

pub use boot::*;
pub use clock::*;
pub use deleted::*;
pub use held::*;
pub use keep::*;
pub use register::*;
pub use retire::*;
pub use settle::*;
pub use start::*;
pub use state::*;
pub use stop_all::*;
pub use supervise::*;
