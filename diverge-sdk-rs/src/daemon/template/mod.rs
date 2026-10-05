//! A template: what an agent, or a tool, is made from, held by its
//! hash.
//!
//! A template is everything about an agent or a tool that is not its
//! name, not the provider it runs on and not its own mounts: the image,
//! the limits, the [resources](crate::daemon::endpoints::resources) it
//! serves over FUSE into every container made from it, the arguments,
//! and — in words, for whoever makes one — what the create has to bring
//! that the template cannot name — so that a template is shareable, the
//! same template on any daemon hashing the same. The shape is one,
//! [`Template`], written once here; which of the two it is for is its
//! `type`, the first member, `"agent"` for an [agent
//! template](crate::daemon::endpoints::agents::templates) and `"tool"`
//! for a [tool template](crate::daemon::endpoints::tools::templates),
//! so the two kinds never hash the same and a template's text says what
//! it is for. Each family makes, lists and deletes its own. The account
//! a container runs under is not a template's but a create's.
//!
//! # The id is the template's hash
//!
//! The lowercase hexadecimal SHA-256 of the template's compact JSON —
//! members in the order [`Template`] declares them, `type` first,
//! absent members omitted, no whitespace — which is the hash Go's
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
