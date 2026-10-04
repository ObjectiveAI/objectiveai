//! What an agent's edit and a tool's edit share.
//!
//! What changes about a container after its create is the same for an
//! agent and a tool: its name, its mounts, the daemon's own tools it
//! holds and how far each reaches, and the agent that deploys its
//! dependencies. [`Edit`] is those, written once, every member optional
//! — absent, the member is as it is; present, it replaces the
//! container's whole — and each family's edit flattens it into its own
//! request beside the reference that says which container.

mod edit;

pub use edit::*;
