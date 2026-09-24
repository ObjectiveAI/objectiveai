//! Volumes — what a provider will let a caller look at, and the whole
//! life of one.
//!
//! [`list`] says which volumes exist; [`stat`] names one and says how
//! much of it is used and what is in it; [`read`] takes one file out
//! of one, [`write`](mod@write) puts one in, and [`filetree`] says what one
//! holds, once; [`create_capacity`] says how
//! large a volume may be made and [`create`] makes one; [`edit_capacity`] says
//! how far one may grow and [`edit`] changes how much it reserves,
//! whether it keeps what is written into it, or both; and [`delete`]
//! destroys it. Every one of them but [`create`] and
//! [`create_capacity`] names a volume rather than describing one: a caller
//! can only ask for what it was offered, and [`create`] is the move
//! that puts something in the offering.
//!
//! # Many mounters, or one editor
//!
//! A volume may be mounted in any number of containers of its caller
//! at once, and nothing examines, reads, writes, walks, resizes or
//! deletes a volume while any container has it. On the server half
//! that is one hold per volume with two modes — shared, taken by a
//! run for its life, and exclusive, taken by a [`stat`], a [`read`],
//! a [`write`](mod@write), a [`filetree`], an [`edit`] or a [`delete`]
//! for its
//! duration — taken by the handlers, never by the provider;
//! [`refusal`] is what a handler answers when the hold cannot be
//! taken and the endpoint has no frame of its own for it. See
//! [`Volume`](crate::server::volume::Volume) for the rule in full.
//! A volume is seen changing in the tree of a container it is
//! mounted in, watched there by the container's proxy or, where the
//! provider keeps a volume out of the proxy's tree, by the provider
//! itself through [`Volume::watch`](crate::server::volume::Volume::watch)
//! — one tree to the caller either way — and seen at rest, mounted
//! nowhere, through a [`filetree`] of its own.
//!
//! # Volumes rather than paths
//!
//! A volume is a directory a provider has DECIDED to offer, under a
//! [`name`](list::server::response::Volume::name). That is the whole
//! access model: a caller never states a host path, because there is
//! no path it could state that a provider would resolve. It names
//! something it was given and descends from there, and `..` is just a
//! name in a component list rather than an instruction.
//!
//! Which is also what they are FOR. A
//! [`VolumeMount`](crate::shared::containers::request::VolumeMount)
//! names one of these and makes it visible inside a laboratory, so
//! what a caller can name and what it can mount are one list rather
//! than two that could disagree.
//!
//! # Who chose the name
//!
//! A provider, for a volume it offers on its own. The caller, for one
//! it asked [`create`] for. Nothing downstream can tell which, and
//! nothing downstream should: a volume is a volume, and the name is
//! the handle either way.
//!
//! # They outlive everything
//!
//! A volume is the only thing in this specification that persists.
//! Every other scope owns what it made — a laboratory dies with its
//! connection, a plugin with its scope — and a volume does not: it
//! survives the scope that created it, every connection the caller
//! holds, and every container that ever mounted it. Only [`delete`]
//! ends one.

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

#[cfg(feature = "server")]
pub mod names;
#[cfg(feature = "server")]
pub mod refusal;
