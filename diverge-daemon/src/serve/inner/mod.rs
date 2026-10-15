//! What a create names beside its template, checked: the account, the
//! providers, the volumes mounted. Shared by the agents and tools
//! creates and edits, which name the same things.
//!
//! Each check answers in the create's own vocabulary — [`Checked`],
//! and [`Mounted`] for the mounts, whose one answer more is a
//! provider's capabilities excluding a mount — so a handler maps the
//! answer to its frame and nothing else.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod checked;
mod mounted;
mod mounts;
mod provider;

pub use account::*;
pub use checked::*;
pub use mounted::*;
pub use mounts::*;
pub use provider::*;
