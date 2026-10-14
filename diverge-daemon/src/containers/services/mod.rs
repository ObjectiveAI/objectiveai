//! Services: the tools the daemon keeps.
//!
//! A service is a tool the daemon keeps running for as long as
//! anything holds it, and retires once nothing has for `idle_seconds`:
//! a dependency tool whose template is a service — one row under its
//! scope key, the agent, the agent's template, or the template alone,
//! made by the first registration that declares it there — or a tool
//! on record whose template's `mode` is a service. What holds its
//! container up is not any run: for either, the agents ATTACHED to it
//! — a dependency tool's by the registrations that declare its
//! template, a tool on record's by `attach` — and for a tool on
//! record, as well, any connect scope open on it. While anything
//! holds it the container is kept running — started at the attach,
//! the registration or the connect that first held it, started again
//! whenever its run ends, and started at every boot of the daemon's
//! while an agent is attached — and once nothing does, its idle clock
//! runs, and `idle_seconds` after the last hold left with none since
//! it is RETIRED: its run stopped and its state let go, its record
//! kept, to be kept up again by the next hold. A dependency tool's
//! row and its database scope go with the row's delete: its agent's,
//! its agent template's, or its template's; a tool on record's
//! keeping ends with the tool's delete.
//!
//! [`settle`] is what a start does when the program declared nothing,
//! the agent detached from every dependency tool; [`keep`] is a
//! service kept up — its [`State`] made, its [`supervise`] task and
//! its [`clock`] started — and [`held`] its hold recomputed from what
//! holds it; [`start`] is one run of a dependency service's container
//! (a tool on record's is the one every tool has); [`retire`] is the
//! end of its keeping; [`boot`] is every service brought up at the
//! daemon's start; [`stop_all`] is the daemon stopping, the
//! supervisors stilled before the runs are stopped so that none comes
//! back; and [`agent_deleted`], [`template_deleted`] and
//! [`tool_deleted`] are what the deletes — an agent's, an agent
//! template's, a dependency template's, a tool's — do to the services
//! whose rows went with them and those they held up.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod boot;
mod clock;
mod deleted;
mod held;
mod keep;
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
pub use retire::*;
pub use settle::*;
pub use start::*;
pub use state::*;
pub use stop_all::*;
pub use supervise::*;
