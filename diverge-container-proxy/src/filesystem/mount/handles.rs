//! The open file handles of a mount: a path each, and nothing more.
//!
//! A handle is a number the kernel quotes on every read, write and
//! release of an open file, and what this keeps for it is the path
//! the file had when it was opened and whether the open was for
//! writing. No bytes: a read is asked as it comes and a write lands as
//! it comes, so there is nothing to hold between them and nothing to
//! store on a close.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use fuser::{Errno, FileHandle};

/// Every open handle, by its number.
pub struct Handles {
    open: Mutex<BTreeMap<u64, Open>>,
    next: AtomicU64,
}

/// One open handle.
struct Open {
    /// The file's path inside the mount; empty on a file mount.
    path: String,
    writable: bool,
}

impl Handles {
    pub fn new() -> Self {
        Handles {
            open: Mutex::new(BTreeMap::new()),
            next: AtomicU64::new(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<u64, Open>> {
        self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Open a handle on `path`.
    pub fn open(&self, path: String, writable: bool) -> FileHandle {
        let fh = self.next.fetch_add(1, Ordering::Relaxed);
        self.lock().insert(fh, Open { path, writable });
        FileHandle(fh)
    }

    /// The handle's path, and whether it may write.
    pub fn get(&self, fh: FileHandle) -> Result<(String, bool), Errno> {
        self.lock()
            .get(&fh.0)
            .map(|handle| (handle.path.clone(), handle.writable))
            .ok_or(Errno::EBADF)
    }

    /// Forget the handle.
    pub fn release(&self, fh: FileHandle) {
        self.lock().remove(&fh.0);
    }

    /// An entry moved: every handle on it, or under it, follows.
    pub fn retarget(&self, from: &str, to: &str) {
        for handle in self.lock().values_mut() {
            if handle.path == from {
                handle.path = to.to_string();
            } else if let Some(rest) = handle.path.strip_prefix(from).and_then(|rest| rest.strip_prefix('/')) {
                handle.path = format!("{to}/{rest}");
            }
        }
    }
}
