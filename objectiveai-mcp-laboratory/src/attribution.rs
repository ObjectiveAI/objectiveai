//! Filesystem-change attribution: *which agent* created or last
//! modified each path.
//!
//! The MCP laboratory is pid 1 in its container and spawns every bash
//! exec, so every writer is a descendant of some agent's exec. We give
//! each exec its own process GROUP (keyed to the calling agent's AIH,
//! [`register_session`]) and run a process-global **fanotify** watch
//! that reports the pid behind each write; the pid resolves to its
//! process group, the group to the AIH. That AIH is stamped into an
//! in-memory `path → attribution` store (read by the `/filetree`
//! builders) and written behind to a `user.objectiveai.*` xattr so it
//! survives a laboratory restart.
//!
//! ## Requires `CAP_SYS_ADMIN` — dormant until the host grants it
//!
//! `fanotify_init` with pid reporting needs `CAP_SYS_ADMIN`, which
//! podman drops by default. Granting it (namespace-scoped under
//! rootless podman, additive, breaks no image) happens later in the
//! laboratory HOST crate at `podman create`. Until then [`init`]'s
//! fanotify setup fails with `EPERM`, attribution stays disabled, and
//! every [`lookup`] returns empty — so the tree's
//! `created_by`/`modified_by` are simply absent, never wrong. This
//! whole module is written and ready; it lights up the moment the
//! capability is present.
//!
//! Non-Linux dev hosts get inert stubs (the laboratory binary only
//! ever runs in a Linux container).
//!
//! Correlation caveat (upgradable later): a process group is inherited
//! across fork, so a build's whole tree attributes correctly, but a
//! daemon that calls `setsid`/`setpgid` escapes its group — a later
//! cgroup-based correlation would close that gap. Concurrency: two
//! agents writing at once are each attributed exactly (the event
//! carries the writer's pid); no temporal guessing.

/// One path's attribution. Absent fields mean "unknown" (e.g. the file
/// predates the watch, or its writer's process group wasn't ours).
#[derive(Clone, Default)]
pub struct Attribution {
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
}

#[cfg(target_os = "linux")]
pub use linux::{init, lookup, register_session, unregister_session};

#[cfg(not(target_os = "linux"))]
pub use stub::{init, lookup, register_session, unregister_session};

/// Inert attribution for non-Linux builds — the crate compiles on a
/// dev host, but attribution only exists in the Linux container.
#[cfg(not(target_os = "linux"))]
mod stub {
    use std::path::Path;

    use super::Attribution;

    pub fn init(_root: &Path) {}
    pub fn register_session(_pgid: i32, _aih: &str) {}
    pub fn unregister_session(_pgid: i32) {}
    pub fn lookup(_path: &Path) -> Attribution {
        Attribution::default()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, OnceLock};

    use dashmap::DashMap;

    use super::Attribution;

    /// The process-global attribution service.
    struct Attributor {
        /// `pgid → AIH`, held for the lifetime of each bash exec.
        sessions: DashMap<i32, String>,
        /// `path → attribution`, populated by the fanotify loop.
        store: DashMap<PathBuf, Attribution>,
    }

    /// Set once by [`init`] IFF the fanotify watch started. `None` ⇒
    /// attribution is disabled (no `CAP_SYS_ADMIN`), and every entry
    /// point is an inert no-op.
    static ATTRIBUTOR: OnceLock<Arc<Attributor>> = OnceLock::new();

    const XATTR_CREATED_BY: &str = "user.objectiveai.created_by";
    const XATTR_MODIFIED_BY: &str = "user.objectiveai.modified_by";

    /// Try to start attribution watching `root`. Best-effort: on any
    /// failure (notably `EPERM` — no `CAP_SYS_ADMIN`) it logs and
    /// leaves attribution disabled.
    pub fn init(root: &Path) {
        match start_fanotify(root) {
            Ok(fan) => {
                let attributor = Arc::new(Attributor {
                    sessions: DashMap::new(),
                    store: DashMap::new(),
                });
                if ATTRIBUTOR.set(attributor.clone()).is_err() {
                    return;
                }
                // A dedicated blocking thread: `read_events` blocks
                // (the group is initialized without `FAN_NONBLOCK`).
                std::thread::Builder::new()
                    .name("oai-attribution".into())
                    .spawn(move || fanotify_loop(fan, attributor))
                    .ok();
                tracing::info!(
                    "attribution: fanotify watch active on {}",
                    root.display()
                );
            }
            Err(e) => {
                tracing::info!(
                    "attribution: fanotify unavailable ({e}) — \
                     created_by/modified_by will be absent until the \
                     host grants CAP_SYS_ADMIN"
                );
            }
        }
    }

    /// Register a bash exec's process group under the calling agent's
    /// AIH, for the exec's lifetime. No-op when attribution is
    /// disabled.
    pub fn register_session(pgid: i32, aih: &str) {
        if let Some(a) = ATTRIBUTOR.get() {
            a.sessions.insert(pgid, aih.to_string());
        }
    }

    /// Drop a finished exec's process group. No-op when disabled.
    pub fn unregister_session(pgid: i32) {
        if let Some(a) = ATTRIBUTOR.get() {
            a.sessions.remove(&pgid);
        }
    }

    /// The attribution recorded for `path` this session. Empty when
    /// disabled or unknown. Reads the in-memory store only — NOT
    /// xattrs — so a full-filesystem tree walk stays a stat-per-entry
    /// (the durable xattr is written behind for external tools and a
    /// future restart reader, not paid for on every walked node).
    pub fn lookup(path: &Path) -> Attribution {
        match ATTRIBUTOR.get() {
            Some(a) => a.store.get(path).map(|r| r.clone()).unwrap_or_default(),
            None => Attribution::default(),
        }
    }

    fn start_fanotify(root: &Path) -> nix::Result<nix::sys::fanotify::Fanotify> {
        use nix::sys::fanotify::{
            EventFFlags, Fanotify, InitFlags, MarkFlags, MaskFlags,
        };
        let fan = Fanotify::init(
            // Notification class (not permission); report the writer's
            // thread/pid so we can map it to a process group.
            InitFlags::FAN_CLASS_NOTIF
                | InitFlags::FAN_REPORT_TID
                | InitFlags::FAN_CLOEXEC,
            EventFFlags::O_RDONLY | EventFFlags::O_CLOEXEC,
        )?;
        // Watch the whole filesystem the root lives on, for content
        // writes, creations, and rename-into-place (atomic saves).
        // `FAN_ONDIR` + `FAN_EVENT_ON_CHILD` so directory entries
        // count. We deliberately do NOT watch `FAN_ATTRIB`, so our own
        // `setxattr` write-behind can't feed back as an event.
        fan.mark(
            MarkFlags::FAN_MARK_ADD | MarkFlags::FAN_MARK_FILESYSTEM,
            MaskFlags::FAN_CREATE
                | MaskFlags::FAN_MODIFY
                | MaskFlags::FAN_MOVED_TO
                | MaskFlags::FAN_ONDIR
                | MaskFlags::FAN_EVENT_ON_CHILD,
            None,
            Some(root),
        )?;
        Ok(fan)
    }

    /// The blocking event loop: read fanotify events, correlate each to
    /// an AIH via its process group, and record it.
    fn fanotify_loop(fan: nix::sys::fanotify::Fanotify, attributor: Arc<Attributor>) {
        use std::os::fd::AsRawFd;

        use nix::sys::fanotify::MaskFlags;
        loop {
            let events = match fan.read_events() {
                Ok(events) => events,
                Err(nix::errno::Errno::EINTR) => continue,
                // A fatal read error ends the loop; attribution goes
                // quiet (lookups keep returning whatever's in the
                // store).
                Err(_) => break,
            };
            for event in events {
                // `fd == None` marks a queue overflow — some events
                // were dropped. Attribution is best-effort, so skip.
                let Some(fd) = event.fd() else { continue };
                let Some(path) = path_of_fd(fd.as_raw_fd()) else {
                    continue;
                };
                let Some(aih) = aih_for_pid(event.pid(), &attributor) else {
                    continue;
                };
                let is_create = event.mask().intersects(
                    MaskFlags::FAN_CREATE | MaskFlags::FAN_MOVED_TO,
                );
                {
                    let mut entry =
                        attributor.store.entry(path.clone()).or_default();
                    if is_create {
                        entry.created_by = Some(aih.clone());
                    }
                    entry.modified_by = Some(aih.clone());
                }
                stamp_xattr(&path, &aih, is_create);
            }
        }
    }

    /// Resolve the real path behind a fanotify event fd via
    /// `/proc/self/fd/<fd>`.
    fn path_of_fd(fd: std::os::fd::RawFd) -> Option<PathBuf> {
        std::fs::read_link(format!("/proc/self/fd/{fd}")).ok()
    }

    /// The AIH whose exec produced this write: pid → process group →
    /// the registered AIH. `None` when the writer's group isn't ours
    /// (external process) or the pid already exited.
    fn aih_for_pid(pid: i32, attributor: &Attributor) -> Option<String> {
        let pgid =
            nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(pid))).ok()?;
        attributor.sessions.get(&pgid.as_raw()).map(|r| r.clone())
    }

    /// Write-behind the attribution to `user.objectiveai.*` xattrs so
    /// it survives a laboratory restart. Best-effort — silently ignores
    /// filesystems that don't support user xattrs (tmpfs pre-6.6, some
    /// bind mounts) and symlinks (user xattrs are forbidden there).
    fn stamp_xattr(path: &Path, aih: &str, is_create: bool) {
        if is_create {
            let _ = xattr::set(path, XATTR_CREATED_BY, aih.as_bytes());
        }
        let _ = xattr::set(path, XATTR_MODIFIED_BY, aih.as_bytes());
    }
}
