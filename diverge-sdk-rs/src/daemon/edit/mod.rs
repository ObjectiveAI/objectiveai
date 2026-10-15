//! What an agent's edit and a tool's edit share.
//!
//! What changes about a container after its create is the same for an
//! agent and a tool: its name, the account it runs under, its mounts,
//! and the agent that deploys its dependencies. [`Edit`] is those,
//! written once, every member an optional [`Change`] — absent, the
//! member is as it is; `delete`, it is taken away; `set`, it is
//! replaced whole — and each family's edit flattens it into its own
//! request beside the reference that says which container.

mod change;
mod edit;

pub use change::*;
pub use edit::*;
