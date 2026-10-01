//! Templates: what an agent is made from, held by its hash.
//!
//! A template is everything about an agent that is not its name, not
//! the provider it runs on and not its own mounts: the image, the
//! limits, the [resources](crate::daemon::endpoints::resources) it
//! serves over FUSE into every agent made from it, and the arguments
//! — so that a template is shareable, the same template on any
//! daemon hashing the same. A caller makes one with [`create`]
//! and names it afterwards by its id — the hash of the template, so
//! the same template made twice is one template, and a caller that
//! holds the template's text knows its id without asking. An
//! [`agent`](super) is created from a template by that id, with a
//! name and mounts of its own; [`list`] names every template the
//! caller has; [`delete`] removes one no agent was made from.
//!
//! # The id is the template's hash
//!
//! The lowercase hexadecimal SHA-256 of the template's compact JSON —
//! members in the order [`Template`] declares them, absent members
//! omitted, no whitespace — which is the hash Go's `dirhash` writes
//! for one file on each line of its summary. Sixty-four characters.
//! The daemon computes it on a create and answers it; a caller may
//! compute it the same way and need not.

mod resource_directory_mount;
mod resource_file_mount;
mod resource_mode;
mod template;

pub use resource_directory_mount::*;
pub use resource_file_mount::*;
pub use resource_mode::*;
pub use template::*;

pub mod create;
pub mod delete;
pub mod list;
