//! Volumes — what a provider will let a caller look at, and the whole
//! life of one.
//!
//! [`list`] says which volumes exist; [`watch`] names one and opens a
//! scope that streams its tree; [`create`] makes one, [`edit`] changes
//! how much it reserves, and [`delete`] destroys it. Every one of them
//! but [`create`] names a volume rather than describing one: a caller
//! can only ask for what it was offered, and [`create`] is the move
//! that puts something in the offering.
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
//! what a caller can watch and what it can mount are one list rather
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
pub mod delete;
pub mod edit;
pub mod list;
pub mod watch;
