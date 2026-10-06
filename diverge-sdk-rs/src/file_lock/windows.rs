//! The lock on Windows: `LockFileEx` on one sentinel byte.
//!
//! Windows byte-range locks are mandatory, not advisory: a locked
//! range cannot be read or written by anyone else. So the lock is
//! taken on ONE byte far past any content the file could have —
//! Windows permits locking past the end of a file — and the file's
//! contents stay readable and writable to everyone while it is held.
//! The kernel lets the lock go on handle close and on process death,
//! as `flock` does.

use std::fs::File;
use std::io;
use std::os::windows::io::AsRawHandle;

use windows_sys::Win32::Foundation::{ERROR_IO_PENDING, ERROR_LOCK_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::{
    LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
};
use windows_sys::Win32::System::IO::OVERLAPPED;

/// The byte that is locked: beyond any plausible content, so a read
/// or a write of the content never collides with the lock.
const OFFSET: u64 = u64::MAX - 1;

/// The `OVERLAPPED` naming the sentinel byte, and no event, so the
/// call is synchronous.
fn overlapped() -> OVERLAPPED {
    // SAFETY: `OVERLAPPED` is plain data, and all-zero is a valid,
    // inert value; only the offset is set, and `hEvent` stays null.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.Anonymous.Anonymous.Offset = OFFSET as u32;
    overlapped.Anonymous.Anonymous.OffsetHigh = (OFFSET >> 32) as u32;
    overlapped
}

/// Take the exclusive lock on the sentinel byte, failing at once
/// instead of waiting when not waiting. `Ok(true)` is the lock had;
/// `Ok(false)` is another holder, answered only when not waiting.
pub(super) fn lock(file: &File, wait: bool) -> io::Result<bool> {
    let mut overlapped = overlapped();
    let mut flags = LOCKFILE_EXCLUSIVE_LOCK;
    if !wait {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    // SAFETY: a live handle, and `overlapped` outlives the synchronous
    // call.
    if unsafe { LockFileEx(file.as_raw_handle() as _, flags, 0, 1, 0, &mut overlapped) } != 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    let held_elsewhere = matches!(
        error.raw_os_error(),
        Some(code) if code == ERROR_LOCK_VIOLATION as i32 || code == ERROR_IO_PENDING as i32
    );
    if !wait && held_elsewhere {
        return Ok(false);
    }
    Err(error)
}

/// Let the sentinel byte go.
pub(super) fn unlock(file: &File) -> io::Result<()> {
    let mut overlapped = overlapped();
    // SAFETY: a live handle, and `overlapped` outlives the call.
    if unsafe { UnlockFileEx(file.as_raw_handle() as _, 0, 1, 0, &mut overlapped) } != 0 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}
