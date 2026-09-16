//! The programs the provider runs, and the one way it runs them.
//!
//! Every program is a `tokio::process::Command`: spawned and awaited
//! without blocking a runtime thread, its stdin closed, its stdout
//! discarded — the tools narrate — and its stderr kept for the error,
//! and killed if the future awaiting it is dropped. [`run`] is that,
//! and nothing in the crate starts a process another way. [`Error`]
//! is what running one fails with: not started, or finished with an
//! exit the caller does not accept.
//!
//! [`podman`] is how the provider speaks to podman and, on macOS and
//! Windows, to the machine podman runs in. [`resize`] is e2fsprogs —
//! `e2fsck` and `resize2fs`, which resize the filesystem in a
//! volume's image — run natively on Linux and through the machine
//! everywhere else.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

pub mod podman;
pub mod resize;

mod error;
mod run;

pub use error::*;
pub use run::*;
