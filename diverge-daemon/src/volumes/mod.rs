//! Volumes: the directories providers hold, reached through the
//! daemon.
//!
//! The daemon keeps no record of a volume. A volume is a provider's,
//! named by the provider's identity and the name the provider lists
//! it under, and known by the provider's listing: the daemon holds one
//! `volumes::list` stream of each connected provider for the
//! connection's life, [`watch`], and keeps what it tells in a
//! [`Mirror`], which [`list`] and [`find`] read — [`Listed`] is one
//! volume with what the daemon adds
//! — the agents and tools whose records name it in their mounts,
//! [`mounters`], and the tags kept on it,
//! [`store::volumes`](crate::store::volumes) — as a list reports it. What the daemon decides is
//! who may, by the grants; whether the volume is HELD now — a running
//! container has it, or a download, an upload or a transfer of the
//! daemon's is on it — and whether it is IN USE for a delete — a record
//! names it, running or not, or an operation is on it — which is
//! [`held`] and [`in_use`]; and the movement of files into and out of
//! it on the daemon's own connections, [`read`] and [`write()`] one
//! file at a time at rest, with [`entry_at`] and [`files_under`] over
//! the tree the provider walks; and the volume made, changed, dropped
//! and measured on its provider, [`create`], [`edit`], [`delete`] and
//! [`stat`]. A provider's refusal is read by its kind into one of
//! [`Fail`]'s three answers.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod fail;
mod held;
mod io;
mod listed;
mod manage;
mod mirror;
mod mounters;
mod provider;
mod tree;
mod watch;

pub use fail::*;
pub use held::*;
pub use io::*;
pub use listed::*;
pub use manage::*;
pub use mirror::*;
pub use mounters::*;
pub use provider::*;
pub use tree::*;
pub use watch::*;
