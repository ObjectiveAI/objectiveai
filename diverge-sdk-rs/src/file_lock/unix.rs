//! The lock on Unix: `flock`.

use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;

/// Take the exclusive `flock` on the file: `LOCK_EX`, with `LOCK_NB`
/// when not waiting. `Ok(true)` is the lock had; `Ok(false)` is
/// another holder, answered only when not waiting; a waiting call
/// interrupted by a signal is made again.
pub(super) fn lock(file: &File, wait: bool) -> io::Result<bool> {
    let operation = if wait { libc::LOCK_EX } else { libc::LOCK_EX | libc::LOCK_NB };
    loop {
        // SAFETY: a live descriptor, and `flock` takes no pointer.
        if unsafe { libc::flock(file.as_raw_fd(), operation) } == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::WouldBlock if !wait => return Ok(false),
            io::ErrorKind::Interrupted if wait => continue,
            _ => return Err(error),
        }
    }
}

/// Let the `flock` go: `LOCK_UN`.
pub(super) fn unlock(file: &File) -> io::Result<()> {
    // SAFETY: a live descriptor, and `flock` takes no pointer.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) } == 0 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}
