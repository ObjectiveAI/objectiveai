//! The daemon's own volumes: the provider's ten exchanges, on the
//! daemon's host.
//!
//! A daemon holds storage of its own, as a provider does, and offers
//! it the same way: named volumes with a size and a persist mode,
//! listed, examined, read, written, walked, created, resized and
//! destroyed through the ten exchanges the provider protocol's
//! [`volumes`](diverge_provider_sdk::endpoints::volumes) defines. Each
//! is here under the daemon's own tag, and is otherwise the
//! provider's: the request is the provider's request behind the
//! daemon's tag, and every answer, every channel and every frame is
//! the provider's type, re-exported, so that what a listing, a stat,
//! a piece of a file, a written answer, a tree, a capacity, a
//! creation, an edit and a deletion mean is stated once, there. Where
//! the provider's rule names a provider, the daemon stands; where it
//! names a container that mounts the volume, an agent's FUSE mount
//! stands — see
//! [`FuseMount`](crate::endpoints::agents::create::client::request::FuseMount),
//! which names a daemon volume and a subtree of it.
//!
//! # The hold is the same
//!
//! A volume may be mounted into any number of the caller's agents at
//! once, and nothing examines, reads, writes, walks, resizes or
//! deletes it while any agent has it: the exclusive hold every
//! in-place verb takes, refused while the volume is mounted anywhere
//! or under another of the six, exactly as the provider's
//! [`Volume`](diverge_provider_sdk::server::volume::Volume) states.
//!
//! # Not a provider's volumes
//!
//! These are the daemon's, on the daemon's host. A provider's volumes
//! are reached through that provider, and an agent mounts one only
//! on the [`Provider`](crate::endpoints::agents::create::client::request::Provider)
//! it is pinned to. The two do not carry across.

pub mod create;
pub mod create_capacity;
pub mod delete;
pub mod edit;
pub mod edit_capacity;
pub mod filetree;
pub mod list;
pub mod read;
pub mod stat;
pub mod write;
