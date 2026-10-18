//! The open file handles of a mount: a path each, and nothing more.
//!
//! A handle is a number the kernel quotes on every read, write and
//! release of an open file, and what this keeps for it is the path
//! the file had when it was opened and whether the open was for
//! writing. No bytes: a read is asked as it comes and a write lands as
//! it comes, so there is nothing to hold between them and nothing to
//! store on a close.
//!
//! Every call here is made from the filesystem's own thread, the one
//! fuser runs the callbacks on, which is no runtime's — so the lock
//! is taken blocking, for a lookup, and never from a task.

use std::collections::BTreeMap;

use fuser::{Errno, FileHandle};
use tokio::sync::Mutex;

/// Every open handle, by its number.
pub struct Handles {
    table: Mutex<Table>,
}

/// What the lock protects: the next number, and every open handle.
struct Table {
    next: u64,
    open: BTreeMap<u64, Open>,
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
            table: Mutex::new(Table {
                next: 1,
                open: BTreeMap::new(),
            }),
        }
    }

    /// Open a handle on `path`.
    pub fn open(&self, path: String, writable: bool) -> FileHandle {
        let mut table = self.table.blocking_lock();
        let fh = table.next;
        table.next += 1;
        table.open.insert(fh, Open { path, writable });
        FileHandle(fh)
    }

    /// The handle's path, and whether it may write.
    pub fn get(&self, fh: FileHandle) -> Result<(String, bool), Errno> {
        self.table
            .blocking_lock()
            .open
            .get(&fh.0)
            .map(|handle| (handle.path.clone(), handle.writable))
            .ok_or(Errno::EBADF)
    }

    /// Forget the handle.
    pub fn release(&self, fh: FileHandle) {
        self.table.blocking_lock().open.remove(&fh.0);
    }

    /// An entry moved: every handle on it, or under it, follows.
    pub fn retarget(&self, from: &str, to: &str) {
        for handle in self.table.blocking_lock().open.values_mut() {
            if handle.path == from {
                handle.path = to.to_string();
            } else if let Some(rest) = handle.path.strip_prefix(from).and_then(|rest| rest.strip_prefix('/')) {
                handle.path = format!("{to}/{rest}");
            }
        }
    }
}
