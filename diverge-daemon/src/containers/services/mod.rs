//! Services: the dependency tools the daemon keeps.
//!
//! A service is a dependency tool whose template is a service: one
//! row under its scope key — the agent, the agent's template, or the
//! template alone — made by the first registration that declares it
//! there and kept by the daemon from then on. What holds its container
//! up is its ATTACHMENTS, not any run: an agent is attached by a
//! registration that declares the template, detached by one that no
//! longer does, and detached by its delete; while any agent is
//! attached the service's container is kept running — started at the
//! registration that made it, started again whenever its run ends, and
//! started at every boot of the daemon's — and once none is, its idle
//! clock runs, and `idle_seconds` after the last attachment left with
//! none since it is RETIRED: its run stopped and its state let go,
//! its row kept, to be kept up again by the next registration that
//! attaches to it. Its row and its database scope go with the row's
//! delete: its agent's, its agent template's, or its template's.
//!
//! [`settle`] is what a start does when the program declared nothing,
//! the agent detached from every tool; [`keep`] is a service kept up —
//! its [`State`] made, its [`supervise`] task and its [`clock`]
//! started — and [`held`] its hold recomputed from its attachments;
//! [`start`] is one run of its container; [`retire`] is the end of its
//! keeping; [`boot`] is every service brought up at the daemon's
//! start; [`stop_all`] is the daemon stopping, the supervisors stilled
//! before the runs are stopped so that none comes back; and
//! [`agent_deleted`] and [`template_deleted`] are what the deletes —
//! an agent's, an agent template's, a dependency template's — do to
//! the services whose rows went with them and those they held up.
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
