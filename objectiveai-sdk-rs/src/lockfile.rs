//! Native OS-level claim-file primitives.
//!
//! Each claim is addressed by `(dir, key)` and is a DOUBLE lock —
//! two files materialized from the escaped key:
//!
//! - the **gate** `<dir>/<escape(key)>.lock` — the real mutex, and
//!   the carrier of the claim's CONTENT;
//! - the **announce** `<dir>/<escape(key)>.live.lock` — taken only
//!   after the content is fully written.
//!
//! Acquisition: lock the gate (contenders race here and lose
//! instantly), write the content under it, then lock the announce.
//! **Held ⇔ both locked.** The flip to "held" is the announce
//! acquisition — a single atomic kernel event that happens strictly
//! after the content is complete, so the observable states are only
//! ever: not held / "acquiring, content in flux, NOT held" / "held,
//! content complete." Stale bytes from a dead predecessor are
//! unreadable-as-held because the successor rewrites them before it
//! announces. Lock order gate→announce and release order
//! announce→gate make announce-without-gate unrepresentable.
//!
//! The module owns the filename escaping (percent-encoding outside
//! `[A-Za-z0-9_-]`, injective — `.` is escaped so no key can imitate
//! the suffixes) and the suffixes; acquisition also creates `dir`
//! if needed.
//!
//! Per-platform liveness mechanics (per file):
//!
//! - **Windows**: persistent file + `LockFileEx` on a sentinel byte.
//!   Lock state ⇔ owner alive. Kill the process by any means → kernel
//!   releases the lock. Subscribe to release via a blocking shared
//!   `LockFileEx` (wakes when no exclusive holder remains). Subscribe
//!   to acquisition via blocking exclusive `LockFileEx`.
//! - **Unix**: persistent file + `flock(LOCK_EX | LOCK_NB)`. Lock
//!   state ⇔ owner alive. Kill the process → kernel releases the
//!   flock. Subscribe to release via blocking `flock(LOCK_SH)`
//!   (wakes when no exclusive holder remains). Subscribe to
//!   acquisition via blocking `flock(LOCK_EX)`.
//!
//! **Lock files carry content.** Acquisition takes the content to
//! publish, and any process can [`read`] it WITHOUT owning the
//! claim. [`try_read`] is certified by a change subscription (a
//! seqlock over file events): arm a watcher before the first
//! held-probe, read, re-probe, drain the watcher — any event or
//! probe flip retries, infinitely (churn eventually stabilizes). A
//! returned `Some(content)` was therefore written by a
//! continuously-live owner and observed complete: lock state and
//! content cannot be out of sync. [`wait_held`] subscribes to the
//! held state itself; [`wait_read`] composes the two.
//!
//! **Dropping a [`LockClaim`] does NOT release it.** The OS objects
//! are held in [`std::mem::ManuallyDrop`], so an acquired claim
//! persists until process death unless explicitly ended via
//! [`LockClaim::release`]. `let _ = try_acquire(..)` therefore
//! means "claim this for the rest of the process's life."
//!
//! Every fallible op is best-effort at the API boundary —
//! [`try_acquire`] returns `Option`, the blocking subscribers
//! return `io::Result`.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use dashmap::DashMap;

/// Process-global registry of locks this process currently holds, keyed
/// by `(dir, key)`. Makes acquisition REENTRANT in-process: a 2nd
/// `try_acquire`/`wait_acquire` for a key this process already owns
/// succeeds instantly (refcounted) rather than conflicting with our own
/// OS lock — both platforms deny a same-process re-acquire at the OS
/// level (Unix flock on a fresh fd; Windows `LockFileEx` on a fresh
/// handle).
///
/// The OS files live HERE (not on the [`LockClaim`]) so they outlive every
/// outstanding claim for the key and are released only when the last
/// claim is `release`d.
///
/// A [`DashMap`] (sharded locking, per-key `entry` atomicity) — never hold
/// one of its guards across an `.await`.
static HELD: LazyLock<DashMap<(PathBuf, String), Entry>> = LazyLock::new(DashMap::new);

enum Entry {
    /// This process acquired the OS lock itself. The registry owns the
    /// gate + announce files; `refs` counts outstanding claims; the OS
    /// lock is released only when `refs` reaches 0.
    Owned {
        gate: std::mem::ManuallyDrop<std::fs::File>,
        announce: std::mem::ManuallyDrop<std::fs::File>,
        refs: usize,
    },
}

/// A handle to a held claim, identified by `(dir, key)`. The OS files
/// live in the process-global [`HELD`] registry; this value is just a key
/// into it. Dropping the value does NOT release the claim — release is
/// explicit ([`Self::release`]), preserving the original "leak until
/// process death unless ended" contract.
pub struct LockClaim {
    dir: PathBuf,
    key: String,
}

impl LockClaim {
    /// The directory this claim was acquired under.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The key this claim was acquired under.
    pub fn key(&self) -> &str {
        &self.key
    }

    fn map_key(&self) -> (PathBuf, String) {
        (self.dir.clone(), self.key.clone())
    }

    /// Release the claim NOW, on purpose. Consumes it. Decrements the
    /// registry refcount and, when it reaches 0, closes the OS files
    /// (announce first — the claim stops being "held" at that single
    /// kernel event — then the gate). Idempotent if the entry is
    /// already gone.
    ///
    /// On Unix the claim FILES deliberately stay on disk — deleting flock
    /// files is racy ([`try_held`] probes lock state, not existence).
    pub fn release(self) -> std::io::Result<()> {
        use dashmap::mapref::entry::Entry as DmEntry;
        // Decrement + remove under the per-key shard lock (atomic). An
        // absent entry (already released) is a no-op.
        if let DmEntry::Occupied(mut occupied) = HELD.entry(self.map_key()) {
            let remove = match occupied.get_mut() {
                Entry::Owned { refs, .. } => {
                    *refs -= 1;
                    *refs == 0
                }
            };
            if remove {
                let Entry::Owned { gate, announce, .. } = occupied.remove();
                let announce = std::mem::ManuallyDrop::into_inner(announce);
                let gate = std::mem::ManuallyDrop::into_inner(gate);
                release_file(announce)?;
                release_file(gate)?;
            }
        }
        Ok(())
    }
}

/// Explicit release of one lock file. Both platforms unlock then close
/// (`UnlockFileEx` / `flock(LOCK_UN)`); the file stays on disk.
fn release_file(file: std::fs::File) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        // Drop the sentinel-byte lock, then close. The file STAYS on
        // disk (persistent, like Unix) — held-ness is the lock, not
        // existence, so a lingering file is inert. Closing the handle
        // would release the lock anyway; `unlock` is the explicit form.
        winlock::unlock(&file);
        drop(file);
        Ok(())
    }
    #[cfg(unix)]
    {
        use nix::fcntl::{FlockArg, flock};
        use std::os::unix::io::AsRawFd;
        flock(file.as_raw_fd(), FlockArg::Unlock).map_err(std::io::Error::from)?;
        // `file` drops here, closing the fd.
        Ok(())
    }
}

/// Try to acquire `(dir, key)` right now, creating `dir` first if
/// needed, and publish `contents` into the gate file. `None` if
/// another live process holds it, or any other open / lock / write
/// failure (a failed step abandons whatever was won so far — the
/// files are not yet wrapped in `ManuallyDrop`, so dropping them
/// genuinely releases).
pub async fn try_acquire(dir: &Path, key: &str, contents: &str) -> Option<LockClaim> {
    use dashmap::mapref::entry::Entry as DmEntry;
    // Create the dir before touching the registry — never hold a DashMap
    // guard across an `.await`.
    tokio::fs::create_dir_all(dir).await.ok()?;
    let map_key = (dir.to_path_buf(), key.to_string());
    // The per-key `entry` makes check-or-acquire-and-insert atomic: a
    // concurrent same-key acquire blocks on the shard then sees our entry
    // instead of conflicting at the OS level.
    match HELD.entry(map_key.clone()) {
        DmEntry::Occupied(mut occupied) => {
            // Reentrant: this process already owns it → succeed instantly
            // (refcounted) instead of conflicting with our own OS lock.
            match occupied.get_mut() {
                Entry::Owned { refs, .. } => *refs += 1,
            }
            Some(LockClaim { dir: map_key.0, key: map_key.1 })
        }
        DmEntry::Vacant(vacant) => {
            // Not held in-process: real OS acquire. `open_claim_file` is
            // non-blocking (try) and synchronous, so running it under the
            // shard lock is fine. A failed step drops `vacant`, releasing
            // the shard lock.
            let mut gate = open_claim_file(&gate_path(dir, key))?;
            write_contents(&mut gate, contents).ok()?;
            let mut announce = open_claim_file(&announce_path(dir, key))?;
            write_beacon(&mut announce).ok()?;
            vacant.insert(Entry::Owned {
                gate: std::mem::ManuallyDrop::new(gate),
                announce: std::mem::ManuallyDrop::new(announce),
                refs: 1,
            });
            Some(LockClaim { dir: map_key.0, key: map_key.1 })
        }
    }
}

/// Acquire `(dir, key)`, blocking until we own it, then publish
/// `contents`. Returns a held [`LockClaim`].
///
/// Blocks on the GATE (the real mutex). The announce should then be
/// free by protocol (announce-without-gate is unrepresentable); if
/// it isn't — some foreign holder — the won gate is abandoned and
/// the whole acquisition retries after that holder clears.
///
/// Same cancellation caveat as [`wait_released`].
pub async fn wait_acquire(
    dir: &Path,
    key: &str,
    contents: &str,
) -> std::io::Result<LockClaim> {
    let map_key = (dir.to_path_buf(), key.to_string());
    // Reentrant: this process already owns it → instant (no blocking wait).
    if let Some(mut entry) = HELD.get_mut(&map_key) {
        match entry.value_mut() {
            Entry::Owned { refs, .. } => *refs += 1,
        }
        return Ok(LockClaim { dir: map_key.0, key: map_key.1 });
    }
    tokio::fs::create_dir_all(dir).await?;
    let gate_path = gate_path(dir, key);
    let announce_path = announce_path(dir, key);
    loop {
        #[cfg(windows)]
        let mut gate = wait_acquire_windows(gate_path.clone()).await?;
        #[cfg(unix)]
        let mut gate = wait_acquire_unix(gate_path.clone()).await?;
        // On failure `gate` drops here un-leaked, releasing it.
        write_contents(&mut gate, contents)?;
        match open_claim_file(&announce_path) {
            Some(mut announce) => {
                write_beacon(&mut announce)?;
                use dashmap::mapref::entry::Entry as DmEntry;
                match HELD.entry(map_key.clone()) {
                    // Dedup: a concurrent in-process acquire registered the
                    // key while we blocked — our freshly-won OS lock is
                    // redundant; refcount the existing entry and release it.
                    DmEntry::Occupied(mut occupied) => {
                        match occupied.get_mut() {
                            Entry::Owned { refs, .. } => *refs += 1,
                        }
                        drop(occupied);
                        let _ = release_file(announce);
                        let _ = release_file(gate);
                        return Ok(LockClaim { dir: map_key.0, key: map_key.1 });
                    }
                    DmEntry::Vacant(vacant) => {
                        vacant.insert(Entry::Owned {
                            gate: std::mem::ManuallyDrop::new(gate),
                            announce: std::mem::ManuallyDrop::new(announce),
                            refs: 1,
                        });
                        return Ok(LockClaim { dir: map_key.0, key: map_key.1 });
                    }
                }
            }
            None => {
                // Foreign announce holder — abandon the gate and
                // retry once the announce clears.
                drop(gate);
                #[cfg(windows)]
                wait_release_windows(announce_path.clone()).await?;
                #[cfg(unix)]
                wait_release_unix(announce_path.clone()).await?;
            }
        }
    }
}

/// How long to sleep between re-probes while a claim is observed
/// mid-flight (gate locked, announce not yet): on the order of a
/// small filesystem write, the operation the holder is in the middle
/// of. (tokio's timer rounds this up to its wheel granularity — the
/// intent is "yield, then look again almost immediately".)
const PARTIAL_STATE_POLL: std::time::Duration = std::time::Duration::from_micros(100);

/// Is some live process currently holding this claim? Held ⇔ BOTH
/// the gate and the announce are locked.
///
/// NEVER reports from a partial state. The observable combinations:
///
/// - gate unlocked → **false**, immediately. (This includes the
///   microseconds-wide owner-death window where the kernel has
///   released the gate lock but not yet the announce lock —
///   lock-release order on process death is unspecified, so
///   announce-without-gate IS transiently representable there, and it
///   correctly reads as "no live owner".)
/// - gate AND announce locked → **true**, immediately.
/// - gate locked, announce not → an acquisition (or release, or
///   death cleanup) is in flight. The holder is a live process
///   actively between two file operations, so the state resolves in
///   microseconds: spin on a tiny sleep until it flips one way or
///   the other. A pathologically stalled peer (suspended mid-
///   acquire) therefore parks this probe instead of producing a
///   spurious answer — waiting is the contract.
///
/// This is what makes "`try_acquire` failed ⇒ an immediately
/// following [`try_read`] sees the winner" a true invariant: probes
/// can no longer observe the winner's gate→announce window as
/// "not held".
pub async fn try_held(dir: &Path, key: &str) -> bool {
    let gate = gate_path(dir, key);
    let announce = announce_path(dir, key);
    loop {
        match (file_locked(&gate), file_locked(&announce)) {
            (false, _) => return false,
            (true, true) => return true,
            (true, false) => tokio::time::sleep(PARTIAL_STATE_POLL).await,
        }
    }
}

/// Try to read the published content of `(dir, key)` right now.
/// `Some(content)` only if the claim is HELD and the content is
/// certified consistent; `None` if not held.
///
/// Certification is a seqlock over file events: arm a change
/// watcher, probe held, read the gate's bytes, probe held again,
/// drain the watcher — any event or probe flip retries, INFINITELY
/// (churn eventually stabilizes; a vanished owner exits through the
/// `None` arm). A returned `Some` was therefore written by a
/// continuously-live owner and observed complete.
pub async fn try_read(dir: &Path, key: &str) -> std::io::Result<Option<String>> {
    let gate = gate_path(dir, key);
    loop {
        // Arm BEFORE the first probe — no blind spot for an
        // ownership turnover to hide in. A watcher that can't even
        // find the file means nothing was ever acquired here.
        let watcher = match ChangeWatcher::arm(dir, &gate) {
            Ok(w) => w,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        if !try_held(dir, key).await {
            return Ok(None);
        }
        let contents = match tokio::fs::read_to_string(&gate).await {
            Ok(c) => c,
            // Vanished mid-read (file externally removed) — re-evaluate.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        if !try_held(dir, key).await {
            // Owner died (the next iteration returns None) or a
            // successor is mid-acquisition (retry until announced).
            continue;
        }
        if watcher.dirty()? {
            continue;
        }
        return Ok(Some(contents));
    }
}

/// Subscription that completes when and ONLY when the claim at
/// `(dir, key)` is HELD (both the gate and the announce locked) —
/// the acquisition-side dual of [`wait_released`]. Returning does not
/// certify anything beyond that instant; pair with [`try_read`] for
/// certified content.
///
/// Fully event-driven: arm a held-watcher, probe, and if not held
/// block on kernel events, then re-evaluate. There is no kernel
/// "wake when someone ELSE acquires" lock primitive (blocking
/// `flock(LOCK_EX)` would acquire it ourselves), so the wake signal
/// is the owner's post-flip BEACON write to the announce file
/// (plus, on Windows, the announce file's creation, which IS the
/// flip). Arming before the probe means a flip can never fall into
/// a blind spot: its beacon event is queued and the block returns
/// immediately.
///
/// Creates `dir` if needed (an empty locks dir is exactly what
/// acquisition would create) so the watcher has something to watch.
pub async fn wait_held(dir: &Path, key: &str) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    let announce = announce_path(dir, key);
    loop {
        let watcher = HeldWatcher::arm(dir, &announce)?;
        if try_held(dir, key).await {
            return Ok(());
        }
        watcher.wait().await?;
    }
}

/// Subscribe to the published content of `(dir, key)`: block until
/// the claim is HELD, then return its certified content. Composed
/// exactly as it reads: [`wait_held`], then [`try_read`]; a `None`
/// (the owner vanished or churned between the two) loops back to
/// [`wait_held`]. Every returned value carries [`try_read`]'s full
/// certification (written by a continuously-live owner, observed
/// complete).
pub async fn wait_read(dir: &Path, key: &str) -> std::io::Result<String> {
    loop {
        wait_held(dir, key).await?;
        if let Some(contents) = try_read(dir, key).await? {
            return Ok(contents);
        }
    }
}

/// Wait until the claim at `(dir, key)` is released. Does not
/// acquire it. Returns when the kernel signals "no live owner
/// remains."
///
/// Blocks on the ANNOUNCE: by protocol it is the last lock taken
/// and the first released, and announce-without-gate is
/// unrepresentable — so announce-unlocked ⇔ not held.
///
/// **Cancellation note** (Unix): the underlying `flock` syscall
/// runs inside [`tokio::task::spawn_blocking`]. Dropping the
/// returned future cancels the awaiting task, but the syscall keeps
/// blocking until the lock is releasable. The blocking thread
/// eventually returns (and is reclaimed by tokio) — one task-pool
/// thread is parked per abandoned wait. Bounded by how many
/// concurrent waiters get abandoned.
pub async fn wait_released(dir: &Path, key: &str) -> std::io::Result<()> {
    let path = announce_path(dir, key);
    #[cfg(windows)]
    {
        wait_release_windows(path).await
    }
    #[cfg(unix)]
    {
        wait_release_unix(path).await
    }
}

/// PIDs of every live process currently holding the claim at
/// `(dir, key)` — the deduplicated UNION across the gate and the
/// announce files. Empty when not held.
///
/// Reads the live OS state, not who originally acquired: a process
/// that has released or transferred its claim does not appear. Only
/// the active exclusive holder(s) are reported — readers and
/// would-be acquirers blocked in `wait_*` are excluded. In-progress
/// transfers are not special-cased; whoever holds a lock right now
/// is reported.
///
/// Per platform:
/// - **Windows**: the Restart Manager (`RmGetList`) lists every
///   process with an open handle to the file. Our owners are the
///   only persistent handle holders, so this is the holder set.
/// - **Linux**: `/proc/locks`, matched by the file's `(dev, inode)`,
///   FLOCK WRITE rows only (shared-lock `try_read`/`try_held` probes
///   are excluded), holder line only (blocked `->` waiters skipped).
/// - **macOS**: `libproc` — processes with the file open, matched by
///   `(dev, inode)`. No per-fd lock bit exists, so this is
///   open-implies-owner; correct for our resident servers (the
///   owner is the lone persistent opener).
pub async fn owners(dir: &Path, key: &str) -> std::io::Result<Vec<u32>> {
    let gate = gate_path(dir, key);
    let announce = announce_path(dir, key);
    tokio::task::spawn_blocking(move || {
        let mut pids = file_owners(&gate)?;
        for pid in file_owners(&announce)? {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
        Ok(pids)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

/// Recursively collect the live owner PIDs of every lockfile in the
/// tree rooted at `root` — the de-duplicated union of [`file_owners`]
/// across every `*.lock` file (both the `.lock` gates and the
/// `.live.lock` announces) found anywhere beneath it.
///
/// This is the whole-subtree analogue of [`owners`]: where `owners`
/// resolves the holders of one `(dir, key)`, this sweeps an entire
/// directory subtree and returns every distinct process holding any
/// lock within it — the basis for a "kill everything rooted at this
/// `OBJECTIVEAI_DIR`" operation. Because it works straight off the
/// on-disk `*.lock` filenames it needs no key round-trip (no
/// un-escaping): the gate path IS what `file_owners` consumes.
///
/// A `root` that does not exist (or vanishes mid-walk) contributes
/// nothing rather than erroring; only an unreadable directory that
/// does exist surfaces its `io::Error`. Symlinked directories are
/// not followed (the `read_dir` entry's file type reports the link,
/// not its target), so the walk cannot loop. The current process is
/// **not** filtered — a caller that must avoid terminating itself
/// drops `std::process::id()` from the result.
pub async fn owners_in_tree(root: &Path) -> std::io::Result<Vec<u32>> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut pids: Vec<u32> = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            for entry in entries {
                let entry = entry?;
                let file_type = entry.file_type()?;
                let path = entry.path();
                if file_type.is_dir() {
                    stack.push(path);
                } else if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".lock"))
                {
                    for pid in file_owners(&path)? {
                        if !pids.contains(&pid) {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
        Ok(pids)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

#[cfg(windows)]
fn file_owners(path: &Path) -> std::io::Result<Vec<u32>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
    use windows_sys::Win32::System::RestartManager::{
        CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList,
        RmRegisterResources, RmStartSession,
    };

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut session: u32 = 0;
    let mut session_key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    // SAFETY: out-params are valid; session_key is the required size.
    if unsafe { RmStartSession(&mut session, 0, session_key.as_mut_ptr()) } != 0 {
        return Ok(Vec::new());
    }
    // Always end the session.
    struct Session(u32);
    impl Drop for Session {
        fn drop(&mut self) {
            // SAFETY: a started session handle.
            unsafe {
                RmEndSession(self.0);
            }
        }
    }
    let _session = Session(session);

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let files = [wide.as_ptr()];
    // SAFETY: one valid null-terminated filename pointer.
    if unsafe {
        RmRegisterResources(
            session,
            1,
            files.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    } != 0
    {
        return Ok(Vec::new());
    }

    // First call sizes the array; ERROR_MORE_DATA is expected.
    let mut needed: u32 = 0;
    let mut count: u32 = 0;
    let mut reason: u32 = 0;
    // SAFETY: null array with zero count is the documented sizing call.
    let rc = unsafe {
        RmGetList(
            session,
            &mut needed,
            &mut count,
            std::ptr::null_mut(),
            &mut reason,
        )
    };
    if rc != 0 && rc != ERROR_MORE_DATA {
        return Ok(Vec::new());
    }
    if needed == 0 {
        return Ok(Vec::new());
    }

    let mut infos: Vec<RM_PROCESS_INFO> =
        vec![unsafe { std::mem::zeroed() }; needed as usize];
    count = needed;
    // SAFETY: `infos` holds `count` writable elements.
    if unsafe {
        RmGetList(
            session,
            &mut needed,
            &mut count,
            infos.as_mut_ptr(),
            &mut reason,
        )
    } != 0
    {
        return Ok(Vec::new());
    }

    let me = std::process::id();
    Ok(infos[..count as usize]
        .iter()
        .map(|i| i.Process.dwProcessId)
        .filter(|&pid| pid != 0 && pid != me)
        .collect())
}

#[cfg(target_os = "linux")]
fn file_owners(path: &Path) -> std::io::Result<Vec<u32>> {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(Vec::new());
    };
    let target_ino = meta.ino();
    let dev = meta.dev();
    let major = (dev >> 8) & 0xfff;
    let minor = (dev & 0xff) | ((dev >> 12) & 0xfff_ff00);

    let Ok(locks) = std::fs::read_to_string("/proc/locks") else {
        return Ok(Vec::new());
    };
    let me = std::process::id();
    let mut pids = Vec::new();
    for line in locks.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // `<id>: FLOCK ADVISORY WRITE <pid> <maj>:<min>:<ino> ...`
        // Blocked waiters render as `<id>: -> FLOCK ...` — f[1] is
        // `->`, so they're skipped by the FLOCK check below.
        if f.len() < 6 || f[1] != "FLOCK" || f[3] != "WRITE" {
            continue;
        }
        let Ok(pid) = f[4].parse::<u32>() else {
            continue;
        };
        if pid == 0 || pid == me {
            continue;
        }
        let mut di = f[5].split(':');
        let (Some(maj), Some(min), Some(ino)) = (di.next(), di.next(), di.next())
        else {
            continue;
        };
        let (Ok(maj), Ok(min), Ok(ino)) = (
            u64::from_str_radix(maj, 16),
            u64::from_str_radix(min, 16),
            ino.parse::<u64>(),
        ) else {
            continue;
        };
        if ino == target_ino && maj == major && min == minor && !pids.contains(&pid)
        {
            pids.push(pid);
        }
    }
    Ok(pids)
}

/// The slice of `<sys/proc_info.h>` the libc crate doesn't export:
/// the `PROC_PIDFDVNODEPATHINFO` flavor structs and two constants.
/// Layouts mirror the headers exactly (`MAXPATHLEN` = 1024).
#[cfg(target_os = "macos")]
mod libproc {
    pub const PROC_ALL_PIDS: u32 = 1;
    pub const PROC_PIDFDVNODEPATHINFO: i32 = 2;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct VinfoStat {
        pub vst_dev: u32,
        pub vst_mode: u16,
        pub vst_nlink: u16,
        pub vst_ino: u64,
        pub vst_uid: u32,
        pub vst_gid: u32,
        pub vst_atime: i64,
        pub vst_atimensec: i64,
        pub vst_mtime: i64,
        pub vst_mtimensec: i64,
        pub vst_ctime: i64,
        pub vst_ctimensec: i64,
        pub vst_birthtime: i64,
        pub vst_birthtimensec: i64,
        pub vst_size: i64,
        pub vst_blocks: i64,
        pub vst_blksize: i32,
        pub vst_flags: u32,
        pub vst_gen: u32,
        pub vst_rdev: u32,
        pub vst_qspare: [i64; 2],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct VnodeInfo {
        pub vi_stat: VinfoStat,
        pub vi_type: i32,
        pub vi_pad: i32,
        pub vi_fsid: [i32; 2],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct VnodeInfoPath {
        pub vip_vi: VnodeInfo,
        pub vip_path: [u8; 1024],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct VnodeFdInfoWithPath {
        pub pvip: VnodeInfoPath,
    }
}

#[cfg(target_os = "macos")]
fn file_owners(path: &Path) -> std::io::Result<Vec<u32>> {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(Vec::new());
    };
    let (target_dev, target_ino) = (meta.dev() as u32, meta.ino());
    let me = std::process::id();

    // List all pids.
    // SAFETY: sizing call (null buffer, zero size) returns the byte
    // count needed.
    let bytes = unsafe {
        nix::libc::proc_listpids(libproc::PROC_ALL_PIDS, 0, std::ptr::null_mut(), 0)
    };
    if bytes <= 0 {
        return Ok(Vec::new());
    }
    let cap = bytes as usize / std::mem::size_of::<i32>();
    let mut all_pids = vec![0i32; cap];
    // SAFETY: buffer sized to `bytes`.
    let got = unsafe {
        nix::libc::proc_listpids(
            libproc::PROC_ALL_PIDS,
            0,
            all_pids.as_mut_ptr() as *mut nix::libc::c_void,
            bytes,
        )
    };
    if got <= 0 {
        return Ok(Vec::new());
    }
    all_pids.truncate(got as usize / std::mem::size_of::<i32>());

    let mut pids = Vec::new();
    for pid in all_pids {
        if pid <= 0 || pid as u32 == me {
            continue;
        }
        // List the process's open fds.
        // SAFETY: sizing call.
        let fbytes = unsafe {
            nix::libc::proc_pidinfo(
                pid,
                nix::libc::PROC_PIDLISTFDS,
                0,
                std::ptr::null_mut(),
                0,
            )
        };
        if fbytes <= 0 {
            continue;
        }
        let fcap = fbytes as usize / std::mem::size_of::<nix::libc::proc_fdinfo>();
        let mut fds: Vec<nix::libc::proc_fdinfo> =
            vec![unsafe { std::mem::zeroed() }; fcap];
        // SAFETY: buffer sized to `fbytes`.
        let fgot = unsafe {
            nix::libc::proc_pidinfo(
                pid,
                nix::libc::PROC_PIDLISTFDS,
                0,
                fds.as_mut_ptr() as *mut nix::libc::c_void,
                fbytes,
            )
        };
        if fgot <= 0 {
            continue;
        }
        fds.truncate(fgot as usize / std::mem::size_of::<nix::libc::proc_fdinfo>());

        for fd in fds {
            if fd.proc_fdtype != nix::libc::PROX_FDTYPE_VNODE as u32 {
                continue;
            }
            let mut vi: libproc::VnodeFdInfoWithPath = unsafe { std::mem::zeroed() };
            // SAFETY: `vi` is a correctly-sized out-struct.
            let n = unsafe {
                nix::libc::proc_pidfdinfo(
                    pid,
                    fd.proc_fd,
                    libproc::PROC_PIDFDVNODEPATHINFO,
                    &mut vi as *mut _ as *mut nix::libc::c_void,
                    std::mem::size_of::<libproc::VnodeFdInfoWithPath>() as i32,
                )
            };
            if n <= 0 {
                continue;
            }
            if vi.pvip.vip_vi.vi_stat.vst_ino == target_ino
                && vi.pvip.vip_vi.vi_stat.vst_dev == target_dev
                && !pids.contains(&(pid as u32))
            {
                pids.push(pid as u32);
            }
        }
    }
    Ok(pids)
}

/// `<dir>/<escape(key)>.lock` — the gate: the real mutex, carries
/// the content.
fn gate_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{}.lock", filename_escape(key)))
}

/// `<dir>/<escape(key)>.live.lock` — the announce: locked last,
/// released first. The escape never emits `.`, so no key's gate can
/// collide with another key's announce.
fn announce_path(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{}.live.lock", filename_escape(key)))
}

/// Why [`spawn_until_published`] failed.
#[derive(Debug)]
pub enum SpawnPublishError {
    /// A lockfile operation failed.
    Lock(std::io::Error),
    /// Spawning the executable failed.
    Spawn(std::io::Error),
    /// The child exited without the lock ever being published (and no
    /// concurrent winner published it either). Carries the child's
    /// captured output for the error report.
    ExitedBeforePublishing {
        name: String,
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },
}

impl std::fmt::Display for SpawnPublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lock(e) => write!(f, "lockfile: {e}"),
            Self::Spawn(e) => write!(f, "spawn: {e}"),
            Self::ExitedBeforePublishing { name, status, stdout, stderr } => write!(
                f,
                "{name} exited ({status}) before publishing its lock; stdout: {stdout}; stderr: {stderr}"
            ),
        }
    }
}

impl std::error::Error for SpawnPublishError {}

/// Lock-based background spawn: the detached-server discipline shared
/// by every `* spawn` flow (api/db/mcp/daemon/laboratories in the CLI,
/// laboratory managers in the viewer shell).
///
/// A server's readiness signal is its lockfile: once up, it claims
/// `(dir, key)` and publishes its client-connect content. The flow:
///
/// 1. [`try_read`] — already held by a live owner ⇒ return its
///    published content without spawning.
/// 2. Otherwise spawn `exe` (caller's `configure` sets args/env; the
///    child inherits the parent environment): null stdin,
///    piped-and-drained stdout/stderr (a child that dies before
///    publishing reports its own output in the error), detached from
///    the console on Windows (`CREATE_NO_WINDOW | DETACHED_PROCESS`),
///    `kill_on_drop` false so the child outlives the caller.
/// 3. Race [`wait_read`] against the child's exit. Published ⇒ return
///    the content. Child exited first ⇒ re-probe (it may have LOST the
///    claim race to a concurrent winner — a held lock now is success);
///    only a dead child AND a free lock is a failure.
pub async fn spawn_until_published(
    exe: &Path,
    dir: &Path,
    key: &str,
    configure: impl FnOnce(&mut tokio::process::Command),
) -> Result<String, SpawnPublishError> {
    if let Some(listening) = try_read(dir, key).await.map_err(SpawnPublishError::Lock)? {
        return Ok(listening);
    }

    let name = exe
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string());

    let mut cmd = tokio::process::Command::new(exe);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008).
        cmd.creation_flags(0x0800_0008);
    }
    configure(&mut cmd);

    let mut child = cmd.spawn().map_err(SpawnPublishError::Spawn)?;

    // Drain both pipes from the moment of spawn: a failing child can
    // spew more than a pipe buffer before exiting, and an undrained
    // pipe would wedge it before it ever reports.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf).await;
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf).await;
        }
        buf
    });

    let listening = tokio::select! {
        read = wait_read(dir, key) => read.map_err(SpawnPublishError::Lock)?,
        status = child.wait() => {
            return match try_read(dir, key).await.map_err(SpawnPublishError::Lock)? {
                Some(listening) => Ok(listening),
                None => {
                    // The dead child's pipes EOF promptly; the timeout
                    // guards a still-living grandchild holding the
                    // write ends open.
                    let drain_timeout = std::time::Duration::from_secs(2);
                    let stdout = match tokio::time::timeout(drain_timeout, stdout_task).await {
                        Ok(Ok(buf)) => buf,
                        _ => Vec::new(),
                    };
                    let stderr = match tokio::time::timeout(drain_timeout, stderr_task).await {
                        Ok(Ok(buf)) => buf,
                        _ => Vec::new(),
                    };
                    // One last probe: the drain gave a concurrent
                    // winner extra time to publish.
                    if let Some(listening) =
                        try_read(dir, key).await.map_err(SpawnPublishError::Lock)?
                    {
                        return Ok(listening);
                    }
                    Err(SpawnPublishError::ExitedBeforePublishing {
                        name,
                        status: status.map_err(SpawnPublishError::Spawn)?,
                        stdout: String::from_utf8_lossy(&stdout).trim().to_string(),
                        stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
                    })
                }
            };
        }
    };

    // Child drops without killing (kill_on_drop false): detached.
    drop(child);

    Ok(listening)
}

/// Invert [`filename_escape`]: `%XX` → byte, everything else verbatim.
/// `None` on malformed escapes (foreign files in the dir).
fn filename_unescape(escaped: &str) -> Option<String> {
    let bytes = escaped.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hi = (hex[0] as char).to_digit(16)?;
            let lo = (hex[1] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Every claim KEY with files in `dir`, recovered from the gate
/// filenames (`<escape(key)>.lock`; the `.live.lock` announces are
/// skipped so each key appears once). Purely an enumeration — no
/// locks are taken, no liveness is implied; pair with [`try_held`].
/// Filenames that aren't gate files or don't unescape are skipped.
pub async fn keys_in_dir(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut keys = Vec::new();
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        // No dir = no keys.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(keys),
        Err(e) => return Err(e),
    };
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.ends_with(".live.lock") {
            continue;
        }
        let Some(escaped) = name.strip_suffix(".lock") else {
            continue;
        };
        if let Some(key) = filename_unescape(escaped) {
            keys.push(key);
        }
    }
    Ok(keys)
}

/// Percent-escape `key` into a filename-safe token: `[A-Za-z0-9_-]`
/// pass through, every other byte (including `.` and `%` itself)
/// becomes `%XX` (uppercase hex). Injective — distinct keys can
/// never collide on disk, and no escaped key can imitate the
/// `.lock` / `.live.lock` suffixes.
fn filename_escape(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for b in key.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The post-flip beacon: one byte written to the announce file AFTER
/// its lock is taken. The flock flip itself emits no file event, so
/// without this a held-subscriber ([`wait_held`]) could arm its
/// watcher, probe not-held, and block forever while the flip slid
/// into the gap after the announce file's last pre-flip event. The
/// beacon guarantees at least one file event lands strictly AFTER
/// "held" becomes true. (On Windows the announce CREATION is itself
/// both the flip and a directory event, but the beacon is kept
/// uniform.) The announce's content is never read — only its lock
/// state and events matter.
fn write_beacon(announce: &mut std::fs::File) -> std::io::Result<()> {
    use std::io::Write;
    announce.write_all(b"1")?;
    announce.flush()
}

/// Truncate-and-write `contents`, so a reused gate file (Unix
/// re-acquisition) never shows a stale suffix.
fn write_contents(file: &mut std::fs::File, contents: &str) -> std::io::Result<()> {
    use std::io::{Seek, Write};
    file.set_len(0)?;
    file.seek(std::io::SeekFrom::Start(0))?;
    file.write_all(contents.as_bytes())?;
    file.flush()
}

/// Is the file at `path` exclusively locked by a live process? Both
/// platforms: a non-blocking SHARED-lock probe (`LockFileEx` /
/// `flock(LOCK_SH)`) — success ⇒ no exclusive holder ⇒ release
/// immediately and report false. A stale/orphaned file (owner dead,
/// lock long gone) therefore reads as NOT held. A missing file is false.
fn file_locked(path: &Path) -> bool {
    #[cfg(windows)]
    {
        let Ok(file) = std::fs::OpenOptions::new().read(true).open(path) else {
            // No file at all — no holder.
            return false;
        };
        if winlock::try_lock(&file, false) {
            // Got the shared lock → no exclusive holder. Release.
            winlock::unlock(&file);
            false
        } else {
            true
        }
    }
    #[cfg(unix)]
    {
        use nix::fcntl::{FlockArg, flock};
        use std::os::unix::io::AsRawFd;
        let Ok(file) = std::fs::OpenOptions::new().read(true).open(path) else {
            // No file at all — no holder.
            return false;
        };
        if flock(file.as_raw_fd(), FlockArg::LockSharedNonblock).is_ok() {
            // Got the shared lock → no exclusive holder. Release.
            let _ = flock(file.as_raw_fd(), FlockArg::Unlock);
            false
        } else {
            true
        }
    }
}

// ---------------------------------------------------------------------
// Change watcher — the [`try_read`] certification primitive. Armed before
// the first held-probe, drained after the read: reports whether
// anything relevant happened in between. Conservative: ambiguity
// reads as dirty → retry.
// ---------------------------------------------------------------------

/// Windows: a directory change notification (names, last-write,
/// size). Watching the whole dir is coarser than one file — events
/// from sibling locks cause harmless retries.
#[cfg(windows)]
struct ChangeWatcher {
    handle: isize,
}

#[cfg(windows)]
impl ChangeWatcher {
    fn arm(dir: &Path, _gate: &Path) -> std::io::Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
            FILE_NOTIFY_CHANGE_SIZE, FindFirstChangeNotificationW,
        };
        let dir_wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `dir_wide` is null-terminated and lives through
        // the call.
        let handle = unsafe {
            FindFirstChangeNotificationW(
                dir_wide.as_ptr(),
                0,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            handle: handle as isize,
        })
    }

    fn dirty(&self) -> std::io::Result<bool> {
        use windows_sys::Win32::Foundation::{WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
        use windows_sys::Win32::System::Threading::WaitForSingleObject;
        // SAFETY: handle valid for the watcher's lifetime; zero
        // timeout = non-blocking poll.
        let rc = unsafe { WaitForSingleObject(self.handle as _, 0) };
        match rc {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(std::io::Error::last_os_error()),
            other => Err(std::io::Error::other(format!(
                "unexpected WaitForSingleObject result: {other}"
            ))),
        }
    }
}

#[cfg(windows)]
impl Drop for ChangeWatcher {
    fn drop(&mut self) {
        use windows_sys::Win32::Storage::FileSystem::FindCloseChangeNotification;
        // SAFETY: handle valid + owned for the watcher's lifetime.
        unsafe {
            FindCloseChangeNotification(self.handle as _);
        }
    }
}

/// Linux: a non-blocking inotify watch on the gate file itself —
/// modify, truncate (attrib), close-write, delete, move.
#[cfg(target_os = "linux")]
struct ChangeWatcher {
    inotify: nix::sys::inotify::Inotify,
}

#[cfg(target_os = "linux")]
impl ChangeWatcher {
    fn arm(_dir: &Path, gate: &Path) -> std::io::Result<Self> {
        use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
        let inotify =
            Inotify::init(InitFlags::IN_NONBLOCK).map_err(std::io::Error::from)?;
        inotify
            .add_watch(
                gate,
                AddWatchFlags::IN_MODIFY
                    | AddWatchFlags::IN_ATTRIB
                    | AddWatchFlags::IN_CLOSE_WRITE
                    | AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF,
            )
            .map_err(std::io::Error::from)?;
        Ok(Self { inotify })
    }

    fn dirty(&self) -> std::io::Result<bool> {
        match self.inotify.read_events() {
            Ok(events) => Ok(!events.is_empty()),
            Err(nix::errno::Errno::EAGAIN) => Ok(false),
            Err(e) => Err(std::io::Error::from(e)),
        }
    }
}

/// Non-Linux Unix (macOS et al): a metadata snapshot of the gate
/// file — inode, size, mtime and ctime at nanosecond granularity.
/// Any in-place rewrite (truncate + write) moves size/mtime/ctime; a
/// replacement file moves the inode. Combined with the held-probes
/// bracketing the read, a false-clean would require a full ownership
/// turnover reproducing identical metadata within one timestamp
/// quantum. A vanished file reads as dirty.
#[cfg(all(unix, not(target_os = "linux")))]
struct ChangeWatcher {
    gate: PathBuf,
    snapshot: (u64, u64, i64, i64, i64, i64),
}

#[cfg(all(unix, not(target_os = "linux")))]
impl ChangeWatcher {
    fn snapshot_of(gate: &Path) -> std::io::Result<(u64, u64, i64, i64, i64, i64)> {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(gate)?;
        Ok((
            meta.ino(),
            meta.size(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec(),
        ))
    }

    fn arm(_dir: &Path, gate: &Path) -> std::io::Result<Self> {
        Ok(Self {
            gate: gate.to_path_buf(),
            snapshot: Self::snapshot_of(gate)?,
        })
    }

    fn dirty(&self) -> std::io::Result<bool> {
        match Self::snapshot_of(&self.gate) {
            Ok(snapshot) => Ok(snapshot != self.snapshot),
            // Vanished — definitely changed.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(e) => Err(e),
        }
    }
}

// ---------------------------------------------------------------------
// Held watcher — the [`wait_read`] blocking primitive. Armed before
// the held-probe; `wait` blocks on kernel events until anything
// relevant happens in the locks dir (acquisitions always emit at
// least the post-flip beacon event). Spurious wakes are fine — the
// caller loops and re-evaluates.
// ---------------------------------------------------------------------

/// Windows: the same directory change notification the release
/// waiter uses, blocked on without a timeout.
#[cfg(windows)]
struct HeldWatcher {
    handle: isize,
}

#[cfg(windows)]
impl HeldWatcher {
    fn arm(dir: &Path, _announce: &Path) -> std::io::Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
            FILE_NOTIFY_CHANGE_SIZE, FindFirstChangeNotificationW,
        };
        let dir_wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `dir_wide` is null-terminated and lives through
        // the call.
        let handle = unsafe {
            FindFirstChangeNotificationW(
                dir_wide.as_ptr(),
                0,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            handle: handle as isize,
        })
    }

    async fn wait(&self) -> std::io::Result<()> {
        use windows_sys::Win32::Foundation::{WAIT_FAILED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};
        let handle = self.handle;
        // `&self` outlives the await, so the handle stays open for
        // the blocked thread.
        tokio::task::spawn_blocking(move || {
            // SAFETY: handle valid for the watcher's lifetime.
            let rc = unsafe { WaitForSingleObject(handle as _, INFINITE) };
            match rc {
                WAIT_OBJECT_0 => Ok(()),
                WAIT_FAILED => Err(std::io::Error::last_os_error()),
                other => Err(std::io::Error::other(format!(
                    "unexpected WaitForSingleObject result: {other}"
                ))),
            }
        })
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
    }
}

#[cfg(windows)]
impl Drop for HeldWatcher {
    fn drop(&mut self) {
        use windows_sys::Win32::Storage::FileSystem::FindCloseChangeNotification;
        // SAFETY: handle valid + owned for the watcher's lifetime.
        unsafe {
            FindCloseChangeNotification(self.handle as _);
        }
    }
}

/// Linux: a BLOCKING inotify watch on the locks dir — child create /
/// modify / close-write / moved-to all wake it; the owner's beacon
/// write is the guaranteed post-flip event.
#[cfg(target_os = "linux")]
struct HeldWatcher {
    inotify: Option<nix::sys::inotify::Inotify>,
}

#[cfg(target_os = "linux")]
impl HeldWatcher {
    fn arm(dir: &Path, _announce: &Path) -> std::io::Result<Self> {
        use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
        let inotify = Inotify::init(InitFlags::empty()).map_err(std::io::Error::from)?;
        inotify
            .add_watch(
                dir,
                AddWatchFlags::IN_CREATE
                    | AddWatchFlags::IN_MODIFY
                    | AddWatchFlags::IN_CLOSE_WRITE
                    | AddWatchFlags::IN_MOVED_TO
                    | AddWatchFlags::IN_ATTRIB,
            )
            .map_err(std::io::Error::from)?;
        Ok(Self {
            inotify: Some(inotify),
        })
    }

    async fn wait(mut self) -> std::io::Result<()> {
        let inotify = self.inotify.take().expect("wait called once");
        tokio::task::spawn_blocking(move || {
            // Blocking read — returns on the first batch of events.
            inotify.read_events().map(|_| ()).map_err(std::io::Error::from)
        })
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
    }
}

/// Non-Linux Unix (macOS et al): a kqueue vnode watch on the locks
/// dir (entry churn — first-ever creations) and, when it exists, the
/// announce file itself (the beacon write). Blocked on without a
/// timeout.
#[cfg(all(unix, not(target_os = "linux")))]
struct HeldWatcher {
    kqueue: nix::sys::event::Kqueue,
    // Watched fds must stay open while the kqueue is blocked on.
    _dir: std::fs::File,
    _announce: Option<std::fs::File>,
}

#[cfg(all(unix, not(target_os = "linux")))]
impl HeldWatcher {
    fn arm(dir: &Path, announce: &Path) -> std::io::Result<Self> {
        use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};
        use std::os::unix::io::AsRawFd;

        let dir_file = std::fs::File::open(dir)?;
        let announce_file = match std::fs::File::open(announce) {
            Ok(f) => Some(f),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };

        let kqueue = Kqueue::new().map_err(std::io::Error::from)?;
        let fflags = FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_EXTEND
            | FilterFlag::NOTE_ATTRIB
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_LINK;
        let mut changes = vec![KEvent::new(
            dir_file.as_raw_fd() as usize,
            EventFilter::EVFILT_VNODE,
            EventFlag::EV_ADD | EventFlag::EV_CLEAR,
            fflags,
            0,
            0,
        )];
        if let Some(f) = &announce_file {
            changes.push(KEvent::new(
                f.as_raw_fd() as usize,
                EventFilter::EVFILT_VNODE,
                EventFlag::EV_ADD | EventFlag::EV_CLEAR,
                fflags,
                0,
                0,
            ));
        }
        // Register only (empty eventlist, zero timeout).
        kqueue
            .kevent(
                &changes,
                &mut [],
                Some(nix::libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                }),
            )
            .map_err(std::io::Error::from)?;
        Ok(Self {
            kqueue,
            _dir: dir_file,
            _announce: announce_file,
        })
    }

    async fn wait(self) -> std::io::Result<()> {
        use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent};
        tokio::task::spawn_blocking(move || {
            let mut events = [KEvent::new(
                0,
                EventFilter::EVFILT_VNODE,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            )];
            // Blocking wait — no timeout.
            self.kqueue
                .kevent(&[], &mut events, None)
                .map(|_| ())
                .map_err(std::io::Error::from)
        })
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
    }
}

// ---------------------------------------------------------------------
// Windows: persistent file + LockFileEx on a sentinel byte (the Win32
// analogue of flock). Held-ness is the lock, not existence; the kernel
// releases it on handle close / process death / reboot. Release and
// acquire subscriptions are blocking shared / exclusive LockFileEx.
// ---------------------------------------------------------------------

/// Sentinel-byte `LockFileEx` helpers — the Win32 twin of the Unix
/// `flock` calls. A claim's held-ness is an exclusive byte-range lock
/// on ONE sentinel byte far past any content; a held-probe is a shared
/// lock. The kernel releases these on handle close / process death /
/// reboot, so held-ness tracks a live owner rather than file existence.
#[cfg(windows)]
mod winlock {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;

    /// A byte offset beyond any plausible content. Windows permits
    /// locking ranges past EOF, so the file stays content-sized and a
    /// concurrent `ReadFile [0, len)` never collides with this lock
    /// (Windows byte-range locks are MANDATORY, unlike advisory flock).
    const OFFSET: u64 = u64::MAX - 1;

    fn overlapped() -> OVERLAPPED {
        // SAFETY: OVERLAPPED is plain data; a zeroed value is valid and
        // inert. We set only the offset (the union's Offset/OffsetHigh
        // arm) and leave hEvent null → the call is synchronous.
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.Anonymous.Anonymous.Offset = OFFSET as u32;
        ov.Anonymous.Anonymous.OffsetHigh = (OFFSET >> 32) as u32;
        ov
    }

    /// Non-blocking lock of the sentinel byte. `exclusive` selects an
    /// exclusive (acquire) vs shared (held-probe) lock. Returns whether
    /// it was granted.
    pub fn try_lock(file: &std::fs::File, exclusive: bool) -> bool {
        let mut ov = overlapped();
        let mut flags = LOCKFILE_FAIL_IMMEDIATELY;
        if exclusive {
            flags |= LOCKFILE_EXCLUSIVE_LOCK;
        }
        // SAFETY: live handle; `ov` outlives the synchronous call.
        unsafe { LockFileEx(file.as_raw_handle() as _, flags, 0, 1, 0, &mut ov) != 0 }
    }

    /// Blocking lock of the sentinel byte. `exclusive` = wait to acquire
    /// (EX); shared (SH) = wait until no exclusive holder remains, i.e.
    /// until the current owner releases OR dies.
    pub fn lock_blocking(file: &std::fs::File, exclusive: bool) -> std::io::Result<()> {
        let mut ov = overlapped();
        let flags = if exclusive { LOCKFILE_EXCLUSIVE_LOCK } else { 0 };
        // SAFETY: live handle; `ov` outlives the call. No
        // FAIL_IMMEDIATELY ⇒ blocks on the synchronous handle.
        if unsafe { LockFileEx(file.as_raw_handle() as _, flags, 0, 1, 0, &mut ov) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// Release the sentinel-byte lock. (Closing the handle also drops
    /// it; this is the explicit form.)
    pub fn unlock(file: &std::fs::File) {
        let mut ov = overlapped();
        // SAFETY: live handle; `ov` outlives the call.
        unsafe {
            UnlockFileEx(file.as_raw_handle() as _, 0, 1, 0, &mut ov);
        }
    }
}

/// Open (creating if absent) the persistent claim file and take its
/// exclusive sentinel-byte lock, non-blocking. `None` when a live
/// process already holds the lock (or the open fails). The file is NOT
/// deleted on close — held-ness is the lock, not the file's existence.
#[cfg(windows)]
fn open_claim_file(path: &Path) -> Option<std::fs::File> {
    // Persistent (no DELETE_ON_CLOSE). std's default Windows share mode
    // is permissive (read|write|delete), so probers/readers can open it
    // concurrently; LockFileEx provides the mutual exclusion.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .ok()?;
    winlock::try_lock(&file, true).then_some(file)
}

#[cfg(windows)]
async fn wait_release_windows(path: PathBuf) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || {
        // Open read-only to probe; a missing file ⇒ nothing to wait for.
        let file = match std::fs::OpenOptions::new().read(true).open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        // Blocking SHARED lock: granted the moment no exclusive holder
        // remains — including when the holder DIES (the kernel releases
        // its lock on handle close / process teardown). Then release the
        // shared lock immediately; the acquire was just the wakeup.
        winlock::lock_blocking(&file, false)?;
        winlock::unlock(&file);
        Ok(())
    })
    .await
    .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

#[cfg(windows)]
async fn wait_acquire_windows(path: PathBuf) -> std::io::Result<std::fs::File> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        // Blocking exclusive acquire of the sentinel byte.
        winlock::lock_blocking(&file, true)?;
        Ok(file)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

// ---------------------------------------------------------------------
// Unix: O_CREAT|O_EXCL + flock, blocking flock for subscriptions.
// ---------------------------------------------------------------------

#[cfg(unix)]
fn open_claim_file(path: &Path) -> Option<std::fs::File> {
    match try_create_locked(path) {
        Ok(file) => return Some(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return None,
    }
    take_existing_lock(path)
}

#[cfg(unix)]
fn try_create_locked(path: &Path) -> std::io::Result<std::fs::File> {
    use nix::fcntl::{FlockArg, flock};
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(path)?;
    if flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock).is_err() {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(std::io::Error::other("flock failed"));
    }
    Ok(file)
}

#[cfg(unix)]
fn take_existing_lock(path: &Path) -> Option<std::fs::File> {
    use nix::fcntl::{FlockArg, flock};
    use std::os::unix::io::AsRawFd;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .ok()?;
    if flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock).is_err() {
        return None;
    }
    Some(file)
}

#[cfg(unix)]
async fn wait_release_unix(path: PathBuf) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || unix_wait_for_release(&path))
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

#[cfg(unix)]
async fn wait_acquire_unix(path: PathBuf) -> std::io::Result<std::fs::File> {
    tokio::task::spawn_blocking(move || unix_wait_for_acquire(&path))
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

/// Block until the exclusive holder of `path` releases — implemented
/// as a blocking `flock(LOCK_SH)` followed by immediate release.
#[cfg(unix)]
fn unix_wait_for_release(path: &Path) -> std::io::Result<()> {
    use nix::fcntl::{FlockArg, flock};
    use std::os::unix::io::AsRawFd;
    // If the file doesn't exist there's nothing to wait for.
    let file = match std::fs::OpenOptions::new().read(true).open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    // Blocking shared-lock acquire. Wakes the moment no exclusive
    // holder remains.
    flock(file.as_raw_fd(), FlockArg::LockShared)
        .map_err(|e| std::io::Error::other(format!("flock LOCK_SH: {e}")))?;
    // Release the shared lock immediately — we don't actually hold
    // anything, the acquire was just the "release notification."
    let _ = flock(file.as_raw_fd(), FlockArg::Unlock);
    Ok(())
}

/// Block until we exclusively hold the lock at `path`. Creates the
/// file if needed.
#[cfg(unix)]
fn unix_wait_for_acquire(path: &Path) -> std::io::Result<std::fs::File> {
    use nix::fcntl::{FlockArg, flock};
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o644)
        .open(path)?;
    // Blocking exclusive acquire.
    flock(file.as_raw_fd(), FlockArg::LockExclusive)
        .map_err(|e| std::io::Error::other(format!("flock LOCK_EX: {e}")))?;
    Ok(file)
}
