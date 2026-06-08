//! Process-owned exclusive claim files keyed by
//! `agent_instance_hierarchy`.
//!
//! Each registered hierarchy is backed by a file at
//! `<root>/<hier-with-'/'-replaced-by-'_'>`. Open semantics differ by
//! platform, but the externally-observable contract is the same:
//!
//! - While the owning process is alive, the file exists on disk and no
//!   other process can create a file by the same name.
//! - When the owning process dies (by any means — panic, abort, SIGKILL,
//!   alt+F4, OOM kill, power loss), the OS releases the claim. On
//!   Windows the kernel deletes the file via `FILE_FLAG_DELETE_ON_CLOSE`.
//!   On Unix the kernel releases the `flock`; the directory entry may
//!   persist briefly until the next [`AgentInstanceRegistry::observe`]
//!   call reclaims it.
//!
//! Every fallible operation is best-effort: [`observe`] returns `()` and
//! silently swallows IO errors. The registry only tracks claims it
//! actually owns.

use std::collections::HashMap;
use std::path::PathBuf;

pub struct AgentInstanceRegistry {
    root: PathBuf,
    open: HashMap<String, ClaimFile>,
}

struct ClaimFile {
    #[allow(dead_code)] // Drop closes the handle; the field anchors the lifetime.
    file: std::fs::File,
    path: PathBuf,
}

impl Drop for ClaimFile {
    fn drop(&mut self) {
        // Windows: FILE_FLAG_DELETE_ON_CLOSE deletes the file when the
        // handle closes (which happens when `file` drops right after
        // this). Nothing to do here.
        // Unix: the kernel will release the flock when the FD closes,
        // but the directory entry persists. Unlink it explicitly so the
        // graceful-shutdown path leaves no stale file behind.
        #[cfg(unix)]
        let _ = std::fs::remove_file(&self.path);
    }
}

impl AgentInstanceRegistry {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            open: HashMap::new(),
        })
    }

    /// Idempotent, best-effort. The first time we see `hier`, try to
    /// open + lock a claim file for it. Repeat calls are no-ops. Any
    /// failure (file already claimed live, illegal chars, ENOSPC, …) is
    /// silently dropped — the registry only tracks claims it really
    /// owns.
    pub fn observe(&mut self, hier: &str) {
        if self.open.contains_key(hier) {
            return;
        }
        let filename = hier.replace('/', "_");
        let path = self.root.join(filename);
        if let Some(file) = open_claim_file(&path) {
            self.open
                .insert(hier.to_string(), ClaimFile { file, path });
        }
    }

    /// Release the claim immediately. After this returns the file is
    /// gone from disk (or, on Unix where the OS didn't auto-delete it,
    /// explicitly unlinked by the Drop impl) and another process can
    /// reclaim the same hierarchy. No-op if `hier` was never observed
    /// or never produced a successful claim.
    pub fn destroy(&mut self, hier: &str) {
        self.open.remove(hier);
    }
}

// ---------------------------------------------------------------------
// Platform-specific open helpers.
// ---------------------------------------------------------------------

#[cfg(windows)]
fn open_claim_file(path: &std::path::Path) -> Option<std::fs::File> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL,
        FILE_FLAG_DELETE_ON_CLOSE, FILE_SHARE_READ,
    };

    // CreateFileW wants a null-terminated wide string.
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: `wide.as_ptr()` is valid for reads of `wide.len()` u16s
    // and null-terminated. `CreateFileW` returns `INVALID_HANDLE_VALUE`
    // on any failure (including the file already existing) — we check
    // that before wrapping.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ,
            std::ptr::null(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_DELETE_ON_CLOSE,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: `handle` is a valid OS handle owned by us and no other
    // `File` references it. Transferring ownership into the std `File`
    // wrapper hands cleanup (and the DELETE_ON_CLOSE trigger) to its
    // `Drop` impl.
    Some(unsafe { std::fs::File::from_raw_handle(handle as _) })
}

#[cfg(unix)]
fn open_claim_file(path: &std::path::Path) -> Option<std::fs::File> {
    // First try atomic O_CREAT | O_EXCL — wins cleanly when no file
    // exists yet. AlreadyExists falls through to the reclaim path.
    match try_create_locked(path) {
        Ok(file) => return Some(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return None,
    }

    // Reclaim path: open the existing file and acquire its exclusive
    // lock. The lock — not the directory entry — is the source of
    // truth on Unix, so we hold the same inode the prior (now-dead)
    // owner had. If a live owner still holds the lock, `flock`
    // LOCK_NB fails and we silently give up.
    //
    // Critical: we keep the lock continuously from acquisition
    // through return. Never unlink + recreate here — doing so opens
    // a TOCTOU window where two concurrent reclaimers can both end
    // up "owning" the hierarchy (one on the new inode, one on the
    // orphan). The lock arbitrates; whoever wins the `flock` race is
    // the sole owner.
    take_existing_lock(path)
}

#[cfg(unix)]
fn try_create_locked(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use nix::fcntl::{FlockArg, flock};
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(path)?;
    if flock(&file, FlockArg::LockExclusiveNonblock).is_err() {
        // LOCK_NB only fails if another live process holds the lock.
        // We just created the file ourselves; best-effort cleanup
        // before bailing so we don't leave a half-claimed file behind.
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(std::io::Error::other("flock failed"));
    }
    Ok(file)
}

#[cfg(unix)]
fn take_existing_lock(path: &std::path::Path) -> Option<std::fs::File> {
    use nix::fcntl::{FlockArg, flock};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .ok()?;
    if flock(&file, FlockArg::LockExclusiveNonblock).is_err() {
        return None;
    }
    Some(file)
}
