//! One exclusive lock per file, across processes, let go on drop.
//!
//! A lock here is the operating system's advisory lock on a file —
//! `flock` on Unix, `LockFileEx` on Windows — taken exclusive and
//! held by a process for as long as it holds the [`Held`] guard. It
//! is what two processes use to take turns at one thing on disk:
//! extracting an install, initializing a cluster, starting a server
//! that must be the only one. Held-ness is the lock, not the file's
//! existence: the kernel lets the lock go when the holder drops the
//! guard, closes the file, or dies, so a crashed holder blocks nobody,
//! and the file itself is never deleted — removing a file others may
//! be opening to lock is a race on every platform, and an empty file
//! left behind costs nothing.
//!
//! [`wait_exclusive`] blocks until the lock is had; [`try_exclusive`]
//! answers at once with `None` when another process holds it. Both
//! create the file if it is absent and leave its contents alone. Where
//! lock files live is the caller's decision, and the callers in this
//! workspace keep every lock beside the thing it guards.
//!
//! # Why the SDK
//!
//! More than one Diverge program takes turns at a directory — the
//! Postgres supervisor at its install and its cluster, the daemon at
//! what it spawns — and one lock shape, in one place, is how they
//! agree. Nothing here crosses a wire; it is in this crate because it
//! is shared, as [`shared`](crate::shared) is.
//!
//! # Exclusive, and only that
//!
//! There is no shared lock, no reader, no published content and no
//! listing of holders. A lock is a turn, and a turn is exclusive.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod error;
mod held;
mod lock;

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

pub use error::*;
pub use held::*;
pub use lock::*;
