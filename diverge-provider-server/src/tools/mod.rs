//! The programs the provider runs, and the one way it runs them.
//!
//! Every program is a `tokio::process::Command`: spawned and awaited
//! without blocking a runtime thread, its stdin closed, its stderr
//! kept for the error, and killed if the future awaiting it is
//! dropped. [`run`] is that, with stdout discarded — the tools
//! narrate; [`capture`] is that with stdout kept — the tools answer,
//! an id, a port, a listing; [`start`] spawns a program that outlives
//! the call and hands back the child, killed when the child is
//! dropped. Nothing in the crate starts a process another way.
//! [`Error`] is what running one fails with: not started, finished
//! with an exit the caller does not accept, or answered in bytes that
//! are not text.
//!
//! [`podman`] is how the provider speaks to podman and, on macOS and
//! Windows, to the machine podman runs in. [`resize`] is e2fsprogs —
//! `e2fsck` and `resize2fs`, which resize the filesystem in a
//! volume's image — and [`mount`] is `mount` and `umount`, which put
//! that filesystem on a directory for podman to bind; both run
//! natively on Linux and through the machine everywhere else.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

pub mod mount;
pub mod podman;
pub mod resize;

mod error;
mod run;

pub use error::*;
pub use run::*;
