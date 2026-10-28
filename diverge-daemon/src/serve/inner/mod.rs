//! What a create names beside its template, checked: the providers,
//! the volumes mounted. Shared by the agents and tools
//! creates and edits, which name the same things.
//!
//! Each check answers in the create's own vocabulary — [`Checked`],
//! and [`Mounted`] for the mounts, whose one answer more is a
//! provider's capabilities excluding a mount — so a handler maps the
//! answer to its frame and nothing else. For an edit of a record
//! whose container runs, [`additions`] is what the patch may do to
//! its mounts — add — and [`runtime_mount`] makes one such addition
//! into the running container, the provider waited for. [`secrets`]
//! checks a record's secrets against its template and the grants,
//! and [`declared`] a template's declaration of them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod additions;
mod checked;
mod mounted;
mod mounts;
mod patched;
mod provider;
mod runtime_mount;
mod secrets;

pub use additions::*;
pub use checked::*;
pub use mounted::*;
pub use mounts::*;
pub use patched::*;
pub use provider::*;
pub use runtime_mount::*;
pub use secrets::*;
