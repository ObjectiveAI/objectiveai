//! Reading one file out of a container.
//!
//! One file, never a directory. Not a limitation — a limitation would
//! imply the alternative worked.
//!
//! # Why there is no directory read
//!
//! Because nothing walking a filesystem from userspace can take a
//! consistent snapshot of one, and folding a walk into a single
//! request would hide that rather than fix it. `tar` is the obvious
//! counter-example and is not one: it makes the same syscalls anyone
//! else would, holds no privilege, and reports `file changed as we
//! read it` precisely because it cannot prevent what it detects.
//!
//! Real snapshots exist — freezing the container's cgroup, or a
//! copy-on-write filesystem — and both are out of reach here. A freeze
//! stops a container that is being read from and talked to at the same
//! time; CoW needs a filesystem and privileges a rootless container
//! will not have.
//!
//! So a directory read would have been N files read at N different
//! instants, wearing one stream as a disguise. Reading them one at a
//! time is the same thing, honestly labelled — and it composes with
//! [`filetree`](crate::shared::filetree), which is already the
//! mechanism that says what exists and what changed while the reads
//! were happening.
//!
//! A volume mounted nowhere is read the same way, one file at a
//! time, by [`volumes::read`](crate::provider::endpoints::volumes::read) — and
//! at rest, so no tear is possible there.

pub mod request;
pub mod response;
