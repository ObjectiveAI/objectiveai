//! Filesystem-change attribution: *which agent* created or last
//! modified each path.
//!
//! ## Disabled — every lookup returns empty
//!
//! The original design ran a process-global **fanotify** watch
//! (`FAN_MARK_FILESYSTEM` + pid reporting) and correlated each write's
//! pid to a bash exec's process group. Empirical probes (2026-07-15,
//! WSL2 6.18 / podman 5.8.4) proved it can never arm in a laboratory
//! container:
//!
//! - **Rootless** (the real environment): the kernel's unprivileged-
//!   fanotify policy ignores namespaced capabilities — `--cap-add
//!   SYS_ADMIN` or not, `fanotify_init` with pid reporting is `EPERM`
//!   and filesystem/mount marks are `ENOTSUP` (only per-inode marks,
//!   which are inotify with extra steps).
//! - **Rootful**: the capability gate opens, but FID-mode filesystem
//!   marks need exportfs file-handle encoding, which overlayfs (without
//!   `nfs_export=on`) and 9p mounts both lack — `ENOTSUP` on every
//!   superblock in play. And dirent events (`FAN_CREATE` et al.)
//!   require an FID-mode group, which a pid-reporting fd-mode group is
//!   not.
//!
//! The `--cap-add SYS_ADMIN` grant that existed solely for this was
//! stripped from the host's `podman create` at the same time: it
//! excluded managed-cloud platforms (Kubernetes baseline/restricted
//! PSA, Fargate, Cloud Run all refuse it) while buying nothing.
//!
//! The API and the bash-exec process-group registration below remain:
//! every writer in the container is a descendant of one of our bash
//! execs, so a future capability-free correlation (e.g. stamping at
//! the exec layer) plugs back in without touching call sites. Until
//! then `created_by`/`modified_by` are simply absent, never wrong.

use std::path::Path;

/// One path's attribution. Absent fields mean "unknown" — currently
/// always (see the module docs).
#[derive(Clone, Default)]
pub struct Attribution {
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
}

/// Register a bash exec's process group under the calling agent's AIH.
/// Inert today; the registration point a future capability-free
/// attribution will correlate through.
pub fn register_session(_pgid: i32, _aih: &str) {}

/// Drop a finished exec's process group. Inert today.
pub fn unregister_session(_pgid: i32) {}

/// The attribution recorded for `path`. Currently always empty (see
/// the module docs).
pub fn lookup(_path: &Path) -> Attribution {
    Attribution::default()
}
