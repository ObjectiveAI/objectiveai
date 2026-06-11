//! Native OS-level claim-file primitives shared between
//! [`crate::websockets::agent_registry`] (the per-spawn lock owner)
//! and [`crate::command::agents::message`] (the
//! delivery-vs-spawn race driver).
//!
//! Each claim is one filesystem path. Each platform uses its
//! strongest native mechanism for "is anyone alive holding this
//! claim right now":
//!
//! - **Windows**: `CreateFileW + CREATE_NEW + FILE_FLAG_DELETE_ON_CLOSE`.
//!   File existence ⇔ owner alive. Drop the [`LockClaim`] (or kill
//!   the process by any means) → kernel deletes the file.
//!   Subscribe to release via [`FindFirstChangeNotificationW`] on
//!   the parent directory.
//! - **Unix**: persistent file + `flock(LOCK_EX | LOCK_NB)`. Lock
//!   state ⇔ owner alive. Drop the [`LockClaim`] (or kill the
//!   process) → kernel releases the flock. Subscribe to release
//!   via blocking `flock(LOCK_SH)` (wakes when no exclusive holder
//!   remains). Subscribe to acquisition via blocking
//!   `flock(LOCK_EX)`.
//!
//! Every fallible op is best-effort at the API boundary —
//! [`try_acquire`] returns `Option`, the blocking subscribers
//! return `io::Result`.

use std::path::{Path, PathBuf};

/// One held lock. Dropping it releases the claim — Windows: file
/// handle close triggers `FILE_FLAG_DELETE_ON_CLOSE`; Unix: FD
/// close releases the `flock`.
pub struct LockClaim {
    #[allow(dead_code)] // Drop closes the handle; field anchors the lifetime.
    file: std::fs::File,
}

impl LockClaim {
    /// Transfer step 1 of 2 — call BEFORE spawning, with the
    /// [`tokio::process::Command`] of the one child that is to own
    /// this lock.
    ///
    /// - **Unix**: attaches a `pre_exec` hook that clears
    ///   `FD_CLOEXEC` on the lock fd *inside the forked child only*.
    ///   Every fd Rust opens is CLOEXEC by default, so no other
    ///   child this process ever spawns inherits the lock — only
    ///   this command's child does. Because `flock` locks belong to
    ///   the open file description (shared by inherited fds), the
    ///   child holds the *same lock* from the instant of spawn — at
    ///   no point is it released and re-acquired.
    /// - **Windows**: no-op. The handle is never inheritable;
    ///   step 2 injects it post-spawn via `DuplicateHandle`.
    ///
    /// The claim must stay alive until [`Self::transfer`] — dropping
    /// it before the spawn releases the lock as usual.
    pub fn prepare_transfer(&self, cmd: &mut tokio::process::Command) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.file.as_raw_fd();
            // SAFETY: the hook runs post-fork pre-exec in the child;
            // `fcntl(F_SETFD)` is async-signal-safe and `fd` is a
            // plain integer valid in the child's inherited fd table.
            unsafe {
                cmd.pre_exec(move || {
                    nix::fcntl::fcntl(
                        fd,
                        nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::empty()),
                    )
                    .map_err(std::io::Error::from)?;
                    Ok(())
                });
            }
        }
        #[cfg(windows)]
        {
            let _ = cmd;
        }
    }

    /// Transfer step 2 of 2 — call AFTER a successful spawn, with the
    /// child from step 1's command. Consumes the claim.
    ///
    /// On `Ok(())` the child is the *sole* owner: the lock lives
    /// exactly as long as the child (kernel cleanup on exit or
    /// crash), the parent retains no handle and no control, and no
    /// other process the parent spawns ever held it. The parent may
    /// die before or after the child without affecting the lock.
    ///
    /// - **Windows**: `DuplicateHandle`s the lock handle directly
    ///   into the child's handle table (surgical — no inheritance),
    ///   then closes the parent's handle. `FILE_FLAG_DELETE_ON_CLOSE`
    ///   fires when the child's last handle closes. Note the
    ///   spawn→transfer window: until this call returns, only the
    ///   parent holds the lock, so a parent crash inside the window
    ///   releases it while the child lives (fail-open, microseconds).
    /// - **Unix**: nothing left to inject — the child already shares
    ///   the lock via the inherited fd (step 1); consuming `self`
    ///   closes the parent's fd, leaving the child sole owner. No
    ///   fail-open window exists on Unix.
    ///
    /// On `Err` the claim is handed back — the parent still owns the
    /// lock and decides what to do (e.g. the child died instantly).
    pub fn transfer(
        self,
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
            let mut injected: HANDLE = std::ptr::null_mut();
            // SAFETY: all handles are live for the duration of the
            // call — ours via `self.file`, the child's via the
            // `&Child` borrow. `injected` is a valid out-pointer.
            let ok = unsafe {
                DuplicateHandle(
                    GetCurrentProcess(),
                    self.file.as_raw_handle() as HANDLE,
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
            // The child's handle table now keeps the file object (and
            // the lock) alive; dropping our handle leaves the child
            // as sole owner.
            drop(self);
            Ok(())
        }
        #[cfg(unix)]
        {
            let _ = child;
            // The child shares the open file description since spawn
            // (step 1 cleared CLOEXEC). Dropping our fd makes it the
            // sole owner.
            drop(self);
            Ok(())
        }
    }
}

/// Try to acquire `path` right now. `None` if another live process
/// holds it, or any other open / lock failure.
pub fn try_acquire(path: &Path) -> Option<LockClaim> {
    open_claim_file(path).map(|file| LockClaim { file })
}

/// Is some live process currently holding this lock?
///
/// Windows: file existence ⇔ liveness (the file is created with
/// `FILE_FLAG_DELETE_ON_CLOSE`, so it only exists when an owner
/// is alive). Check `path.exists()`.
///
/// Unix: lock state ⇔ liveness. Attempt a non-blocking shared
/// lock; if it succeeds, no exclusive holder remains, so we
/// release immediately and report "not held." If acquisition
/// fails (`EWOULDBLOCK`), an exclusive holder is alive.
pub fn is_held(path: &Path) -> bool {
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

/// Wait until the lock at `path` is released. Does not acquire
/// it. Returns when the kernel signals "no live owner remains."
///
/// On Windows the signal is "the file got deleted from the parent
/// directory." On Unix the signal is "a blocking shared lock
/// succeeded, which only happens when no exclusive holder remains."
///
/// **Cancellation note** (Unix): the underlying `flock` syscall
/// runs inside [`tokio::task::spawn_blocking`]. Dropping the
/// returned future cancels the awaiting task, but the syscall keeps
/// blocking until the lock is releasable. The blocking thread
/// eventually returns (and is reclaimed by tokio) — one task-pool
/// thread is parked per abandoned wait. Bounded by how many
/// concurrent waiters get abandoned.
pub async fn wait_release(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        wait_release_windows(path.to_path_buf()).await
    }
    #[cfg(unix)]
    {
        wait_release_unix(path.to_path_buf()).await
    }
}

/// Acquire the lock at `path`, blocking until we own it. Returns
/// a held [`LockClaim`].
///
/// On Windows this loops `try_acquire`, waiting on
/// [`wait_release`] between attempts. On Unix it's a single
/// blocking `flock(LOCK_EX)` call.
///
/// Same cancellation caveat as [`wait_release`].
pub async fn wait_acquire(path: &Path) -> std::io::Result<LockClaim> {
    #[cfg(windows)]
    {
        wait_acquire_windows(path.to_path_buf()).await
    }
    #[cfg(unix)]
    {
        wait_acquire_unix(path.to_path_buf()).await
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
async fn wait_acquire_windows(path: PathBuf) -> std::io::Result<LockClaim> {
    loop {
        if let Some(claim) = try_acquire(&path) {
            return Ok(claim);
        }
        // No file yet? `try_acquire` failed because the file
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
    use std::os::unix::io::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;

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
async fn wait_acquire_unix(path: PathBuf) -> std::io::Result<LockClaim> {
    tokio::task::spawn_blocking(move || unix_wait_for_acquire(&path))
        .await
        .map_err(|e| std::io::Error::other(format!("join: {e}")))?
        .map(|file| LockClaim { file })
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
    use std::os::unix::io::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;

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
