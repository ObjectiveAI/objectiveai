//! A template: what an agent, or a tool, is made from, held by its
//! hash.
//!
//! A template is everything about an agent or a tool that is not its
//! name, not the provider it runs on and not its own mounts: the
//! image, the limits, the [resources](crate::daemon::endpoints::resources)
//! it serves over FUSE into every container made from it, and the
//! arguments — so that a template is shareable, the same template on
//! any daemon hashing the same. The shape is one, [`Template`],
//! written once here; what is the family's own leads it, the `type`
//! first of all, `"agent"` for an
//! [agent template](crate::daemon::endpoints::agents::templates) —
//! followed by the daemon's [`builtin`](crate::daemon::builtin) tools
//! the agents made from it hold, when it names any — and `"tool"`
//! for a [tool template](crate::daemon::endpoints::tools::templates),
//! so the two kinds never hash the same and a template's text says
//! what it is for. Each family makes, lists and deletes its own.
//!
//! # The id is the template's hash
//!
//! The lowercase hexadecimal SHA-256 of the template's compact JSON —
//! members in the order [`Template`] declares them, the family's own
//! first and `type` the first of those, absent members omitted, no
//! whitespace — which is the hash Go's
//! `dirhash` writes for one file on each line of its summary.
//! Sixty-four characters. The daemon computes it on a create and
//! answers it; a caller may compute it the same way and need not.

mod resource_directory_mount;
mod resource_file_mount;
mod resource_mode;
mod template;

pub use resource_directory_mount::*;
pub use resource_file_mount::*;
pub use resource_mode::*;
pub use template::*;
