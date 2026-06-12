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
//! - **Windows**: `CreateFileW + CREATE_NEW + FILE_FLAG_DELETE_ON_CLOSE`.
//!   File existence ⇔ owner alive. Kill the process by any means →
//!   kernel deletes the file. Subscribe to release via
//!   [`FindFirstChangeNotificationW`] on the parent directory.
//! - **Unix**: persistent file + `flock(LOCK_EX | LOCK_NB)`. Lock
//!   state ⇔ owner alive. Kill the process → kernel releases the
//!   flock. Subscribe to release via blocking `flock(LOCK_SH)`
//!   (wakes when no exclusive holder remains). Subscribe to
//!   acquisition via blocking `flock(LOCK_EX)`.
//!
//! **Lock files carry content.** Acquisition takes the content to
//! publish, and any process can [`read`] it WITHOUT owning the
//! claim. [`read`] is certified by a change subscription (a seqlock
//! over file events): arm a watcher before the first held-probe,
//! read, re-probe, drain the watcher — any event or probe flip
//! retries, infinitely (churn eventually stabilizes). A returned
//! `Some(content)` was therefore written by a continuously-live
//! owner and observed complete: lock state and content cannot be
//! out of sync.
//!
//! **Dropping a [`LockClaim`] does NOT release it.** The OS objects
//! are held in [`std::mem::ManuallyDrop`], so an acquired claim
//! persists until process death unless explicitly ended via
//! [`LockClaim::release`] or handed to a child via
//! [`LockClaim::transfer`]. `let _ = try_acquire(..)` therefore
//! means "claim this for the rest of the process's life."
//!
//! Every fallible op is best-effort at the API boundary —
//! [`try_acquire`] returns `Option`, the blocking subscribers
//! return `io::Result`.

use std::path::{Path, PathBuf};

/// One held claim (gate + announce). Dropping the value does NOT
/// release it — the handles/fds are deliberately leaked
/// (`ManuallyDrop`), so the claim outlives the value and ends only
/// at process death, [`Self::release`], or [`Self::transfer`].
pub struct LockClaim {
    gate: std::mem::ManuallyDrop<std::fs::File>,
    announce: std::mem::ManuallyDrop<std::fs::File>,
}

impl LockClaim {
    fn new(gate: std::fs::File, announce: std::fs::File) -> Self {
        Self {
            gate: std::mem::ManuallyDrop::new(gate),
            announce: std::mem::ManuallyDrop::new(announce),
        }
    }

    /// Transfer step 1 of 2 — call BEFORE spawning, with the
    /// [`tokio::process::Command`] of the one child that is to own
    /// this claim.
    ///
    /// - **Unix**: attaches a `pre_exec` hook that clears
    ///   `FD_CLOEXEC` on both lock fds *inside the forked child
    ///   only*. Every fd Rust opens is CLOEXEC by default, so no
    ///   other child this process ever spawns inherits the claim —
    ///   only this command's child does. Because `flock` locks
    ///   belong to the open file description (shared by inherited
    ///   fds), the child holds the *same locks* from the instant of
    ///   spawn — at no point are they released and re-acquired.
    /// - **Windows**: no-op. The handles are never inheritable;
    ///   step 2 injects them post-spawn via `DuplicateHandle`.
    ///
    /// The claim must stay alive until [`Self::transfer`].
    pub fn prepare_transfer(&self, cmd: &mut tokio::process::Command) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let gate_fd = self.gate.as_raw_fd();
            let announce_fd = self.announce.as_raw_fd();
            // SAFETY: the hook runs post-fork pre-exec in the child;
            // `fcntl(F_SETFD)` is async-signal-safe and both fds are
            // plain integers valid in the child's inherited fd table.
            unsafe {
                cmd.pre_exec(move || {
                    for fd in [gate_fd, announce_fd] {
                        nix::fcntl::fcntl(
                            fd,
                            nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::empty()),
                        )
                        .map_err(std::io::Error::from)?;
                    }
                    Ok(())
                });
            }
        }
        #[cfg(windows)]
        {
            let _ = cmd;
        }
    }

    /// Release the claim NOW, on purpose. Consumes it — and is the
    /// ONLY in-process way to end it (dropping leaks the handles by
    /// design; the claim would persist until process death).
    ///
    /// Order: announce first (the claim stops being "held" at that
    /// single kernel event), then the gate. On Unix the claim FILES
    /// deliberately stay on disk — deleting flock files is racy (a
    /// waiter holding the old inode plus a fresh creator at the same
    /// path would yield two "owners"), and [`is_held`] probes lock
    /// state, not existence.
    pub fn release(mut self) -> std::io::Result<()> {
        // SAFETY: sole take — `self` is consumed and its (no-op)
        // drop glue never touches the fields again.
        let announce = unsafe { std::mem::ManuallyDrop::take(&mut self.announce) };
        let gate = unsafe { std::mem::ManuallyDrop::take(&mut self.gate) };
        release_file(announce)?;
        release_file(gate)
    }

    /// Transfer step 2 of 2 — call AFTER a successful spawn, with
    /// the child from step 1's command. Consumes the claim.
    ///
    /// On `Ok(())` the child is the *sole* owner: the claim lives
    /// exactly as long as the child (kernel cleanup on exit or
    /// crash), the parent retains no handles and no control, and no
    /// other process the parent spawns ever held it. The parent may
    /// die before or after the child without affecting the claim.
    ///
    /// - **Windows**: `DuplicateHandle`s both lock handles directly
    ///   into the child's handle table (surgical — no inheritance),
    ///   then closes the parent's. `FILE_FLAG_DELETE_ON_CLOSE` fires
    ///   when the child's last handles close. Note the
    ///   spawn→transfer window: until this call returns, only the
    ///   parent holds the claim, so a parent crash inside the window
    ///   releases it while the child lives (fail-open,
    ///   microseconds). On `Err` the claim is handed back; if the
    ///   failure was partway through injection, the child co-holds
    ///   the already-injected handle until it exits.
    /// - **Unix**: nothing left to inject — the child already shares
    ///   both locks via the inherited fds (step 1); this call closes
    ///   the parent's fds (a close is NOT an unlock), leaving the
    ///   child sole owner. No fail-open window exists on Unix.
    pub fn transfer(
        mut self,
        child: &tokio::process::Child,
    ) -> Result<(), (Self, std::io::Error)> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Foundation::{
                DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE,
            };
            use windows_sys::Win32::System::Threading::GetCurrentProcess;

            let Some(child_handle) = child.raw_handle() else {
                return Err((
                    self,
                    std::io::Error::other(
                        "child has no process handle (already reaped)",
                    ),
                ));
            };
            for source in [self.gate.as_raw_handle(), self.announce.as_raw_handle()] {
                let mut injected: HANDLE = std::ptr::null_mut();
                // SAFETY: all handles are live for the duration of
                // the call — ours via `self`, the child's via the
                // `&Child` borrow. `injected` is a valid out-pointer.
                let ok = unsafe {
                    DuplicateHandle(
                        GetCurrentProcess(),
                        source as HANDLE,
                        child_handle as HANDLE,
                        &mut injected,
                        0,
                        0, // bInheritHandle = FALSE — children of the child don't get it
                        DUPLICATE_SAME_ACCESS,
                    )
                };
                if ok == 0 {
                    return Err((self, std::io::Error::last_os_error()));
                }
            }
            // The child's handle table now keeps both file objects
            // (and the claim) alive; explicitly closing our handles
            // (drop is a leak under ManuallyDrop, not a close) leaves
            // the child sole owner.
            // SAFETY: sole take — `self` is consumed on this path.
            unsafe {
                drop(std::mem::ManuallyDrop::take(&mut self.announce));
                drop(std::mem::ManuallyDrop::take(&mut self.gate));
            }
            Ok(())
        }
        #[cfg(unix)]
        {
            let _ = child;
            // The child shares both open file descriptions since
            // spawn (step 1 cleared CLOEXEC). Explicitly closing our
            // fds (drop is a leak under ManuallyDrop; and a close is
            // NOT an unlock) makes the child sole owner.
            // SAFETY: sole take — `self` is consumed on this path.
            unsafe {
                drop(std::mem::ManuallyDrop::take(&mut self.announce));
                drop(std::mem::ManuallyDrop::take(&mut self.gate));
            }
            Ok(())
        }
    }
}

/// Explicit, error-surfacing close of one lock file. Windows: the
/// checked `CloseHandle` fires `DELETE_ON_CLOSE`. Unix: explicit
/// `flock(LOCK_UN)` then close.
fn release_file(file: std::fs::File) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::IntoRawHandle;
        use windows_sys::Win32::Foundation::CloseHandle;
        let handle = file.into_raw_handle();
        // SAFETY: `into_raw_handle` transferred ownership to us;
        // this is the sole close of a live handle.
        if unsafe { CloseHandle(handle as _) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
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
    tokio::fs::create_dir_all(dir).await.ok()?;
    let mut gate = open_claim_file(&gate_path(dir, key))?;
    write_contents(&mut gate, contents).ok()?;
    let announce = open_claim_file(&announce_path(dir, key))?;
    Some(LockClaim::new(gate, announce))
}

/// Acquire `(dir, key)`, blocking until we own it, then publish
/// `contents`. Returns a held [`LockClaim`].
///
/// Blocks on the GATE (the real mutex). The announce should then be
/// free by protocol (announce-without-gate is unrepresentable); if
/// it isn't — some foreign holder — the won gate is abandoned and
/// the whole acquisition retries after that holder clears.
///
/// Same cancellation caveat as [`wait_release`].
pub async fn wait_acquire(
    dir: &Path,
    key: &str,
    contents: &str,
) -> std::io::Result<LockClaim> {
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
            Some(announce) => return Ok(LockClaim::new(gate, announce)),
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

/// Is some live process currently holding this claim? Held ⇔ BOTH
/// the gate and the announce are locked — a gate-only state is an
/// acquisition in progress (content in flux) and reports false.
pub fn is_held(dir: &Path, key: &str) -> bool {
    file_locked(&gate_path(dir, key)) && file_locked(&announce_path(dir, key))
}

/// Read the published content of `(dir, key)`. `Some(content)` only
/// if the claim is HELD and the content is certified consistent;
/// `None` if not held.
///
/// Certification is a seqlock over file events: arm a change
/// watcher, probe held, read the gate's bytes, probe held again,
/// drain the watcher — any event or probe flip retries, INFINITELY
/// (churn eventually stabilizes; a vanished owner exits through the
/// `None` arm). A returned `Some` was therefore written by a
/// continuously-live owner and observed complete.
pub async fn read(dir: &Path, key: &str) -> std::io::Result<Option<String>> {
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
        if !is_held(dir, key) {
            return Ok(None);
        }
        let contents = match tokio::fs::read_to_string(&gate).await {
            Ok(c) => c,
            // Vanished mid-read (Windows: owner died) — re-evaluate.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        if !is_held(dir, key) {
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
pub async fn wait_release(dir: &Path, key: &str) -> std::io::Result<()> {
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

/// Truncate-and-write `contents`, so a reused gate file (Unix
/// re-acquisition) never shows a stale suffix.
fn write_contents(file: &mut std::fs::File, contents: &str) -> std::io::Result<()> {
    use std::io::{Seek, Write};
    file.set_len(0)?;
    file.seek(std::io::SeekFrom::Start(0))?;
    file.write_all(contents.as_bytes())?;
    file.flush()
}

/// Is the file at `path` exclusively locked by a live process?
/// Windows: existence ⇔ liveness (`FILE_FLAG_DELETE_ON_CLOSE`).
/// Unix: non-blocking shared-lock probe (success ⇒ no exclusive
/// holder ⇒ release immediately and report false).
fn file_locked(path: &Path) -> bool {
    #[cfg(windows)]
    {
        path.exists()
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
// Change watcher — the [`read`] certification primitive. Armed before
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
// Windows: CreateFileW + DELETE_ON_CLOSE, FindFirstChangeNotificationW
// for release subscription.
// ---------------------------------------------------------------------

#[cfg(windows)]
fn open_claim_file(path: &Path) -> Option<std::fs::File> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL,
        FILE_FLAG_DELETE_ON_CLOSE, FILE_SHARE_READ,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: `wide.as_ptr()` is valid for `wide.len()` u16s and
    // null-terminated. `CreateFileW` returns `INVALID_HANDLE_VALUE`
    // on any failure (including file-already-exists).
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
    // SAFETY: handle is exclusively owned, no aliasing.
    Some(unsafe { std::fs::File::from_raw_handle(handle as _) })
}

#[cfg(windows)]
async fn wait_release_windows(path: PathBuf) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || windows_wait_for_file_gone(&path))
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
}

#[cfg(windows)]
async fn wait_acquire_windows(path: PathBuf) -> std::io::Result<std::fs::File> {
    loop {
        if let Some(file) = open_claim_file(&path) {
            return Ok(file);
        }
        // No file yet? `open_claim_file` failed because the file
        // already exists (someone holds it) — wait for it to be
        // deleted, then retry. If the file truly didn't exist
        // (any other reason `CREATE_NEW` would fail), the wait
        // returns immediately on the first directory change.
        wait_release_windows(path.clone()).await?;
    }
}

/// Block the calling thread on `FindFirstChangeNotificationW` over
/// the parent directory, looping until `path` no longer exists.
#[cfg(windows)]
fn windows_wait_for_file_gone(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{
        INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_NOTIFY_CHANGE_FILE_NAME, FindCloseChangeNotification,
        FindFirstChangeNotificationW, FindNextChangeNotification,
    };
    use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};

    let parent = match path.parent() {
        Some(p) => p,
        None => return Ok(()),
    };
    if !path.exists() {
        return Ok(());
    }

    let parent_wide: Vec<u16> = parent
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: `parent_wide` is null-terminated and lives through
    // the call.
    let handle = unsafe {
        FindFirstChangeNotificationW(
            parent_wide.as_ptr(),
            0,
            FILE_NOTIFY_CHANGE_FILE_NAME,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }

    // RAII guard so the handle is always closed.
    struct Guard(isize);
    impl Drop for Guard {
        fn drop(&mut self) {
            // SAFETY: handle valid + owned for the guard's lifetime.
            unsafe {
                FindCloseChangeNotification(self.0 as _);
            }
        }
    }
    let _guard = Guard(handle as isize);

    loop {
        if !path.exists() {
            return Ok(());
        }
        // SAFETY: handle still valid (held by guard).
        let rc = unsafe { WaitForSingleObject(handle as _, INFINITE) };
        if rc == WAIT_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        if rc != WAIT_OBJECT_0 {
            return Err(std::io::Error::other(format!(
                "unexpected WaitForSingleObject result: {rc}"
            )));
        }
        // SAFETY: re-arm the notification handle for the next round.
        unsafe { FindNextChangeNotification(handle as _) };
    }
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
