//! The open file handles of a mount: each a buffer of its own.
//!
//! A handle is opened on a snapshot — the file's bytes as the caller
//! answered them, or empty for a truncating or creating open — and
//! every read and write works that buffer. A changed buffer is stored
//! whole with the caller on `flush`, `fsync` and `release`, through
//! the store the filesystem hands in; an unchanged one costs nothing.
//! Two write handles each store the whole buffer, and the last close
//! wins.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use fuser::{Errno, FileHandle};

/// Every open handle, by its number.
pub struct Handles {
    open: Mutex<BTreeMap<u64, Open>>,
    next: AtomicU64,
}

/// One open handle: its own copy of the file.
struct Open {
    /// The file's path inside the mount; empty on a file mount.
    path: String,
    buffer: Vec<u8>,
    writable: bool,
    dirty: bool,
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

    /// Open a handle on `buffer`, dirty when it already differs from
    /// the caller's file (a truncating open).
    pub fn open(&self, path: String, buffer: Vec<u8>, writable: bool, dirty: bool) -> FileHandle {
        let fh = self.next.fetch_add(1, Ordering::Relaxed);
        self.lock().insert(
            fh,
            Open {
                path,
                buffer,
                writable,
                dirty,
            },
        );
        FileHandle(fh)
    }

    /// The handle's buffer length.
    pub fn size(&self, fh: FileHandle) -> Result<u64, Errno> {
        self.lock()
            .get(&fh.0)
            .map(|handle| handle.buffer.len() as u64)
            .ok_or(Errno::EBADF)
    }

    /// A slice of the buffer, bounds clamped.
    pub fn read(&self, fh: FileHandle, offset: u64, size: u32) -> Result<Vec<u8>, Errno> {
        let open = self.lock();
        let handle = open.get(&fh.0).ok_or(Errno::EBADF)?;
        let start = (offset as usize).min(handle.buffer.len());
        let end = start.saturating_add(size as usize).min(handle.buffer.len());
        Ok(handle.buffer[start..end].to_vec())
    }

    /// Write into the buffer at `offset`, extending it as needed; the
    /// handle is dirty after.
    pub fn write(&self, fh: FileHandle, offset: u64, data: &[u8]) -> Result<u32, Errno> {
        let mut open = self.lock();
        let handle = open.get_mut(&fh.0).ok_or(Errno::EBADF)?;
        if !handle.writable {
            return Err(Errno::EBADF);
        }
        let start = offset as usize;
        let end = start + data.len();
        if handle.buffer.len() < end {
            handle.buffer.resize(end, 0);
        }
        handle.buffer[start..end].copy_from_slice(data);
        handle.dirty = true;
        Ok(data.len() as u32)
    }

    /// Resize the handle's buffer; the handle is dirty after.
    pub fn resize(&self, fh: FileHandle, len: usize) -> Result<(), Errno> {
        let mut open = self.lock();
        let handle = open.get_mut(&fh.0).ok_or(Errno::EBADF)?;
        handle.buffer.resize(len, 0);
        handle.dirty = true;
        Ok(())
    }

    /// Resize every writable handle on `path`, so a later flush does
    /// not resurrect a length the file no longer has.
    pub fn resize_writable(&self, path: &str, len: usize) {
        for handle in self.lock().values_mut() {
            if handle.writable && handle.path == path {
                handle.buffer.resize(len, 0);
            }
        }
    }

    /// Store a changed handle's buffer through `store`, and mark it
    /// clean. An unchanged handle is nothing to do. The store runs
    /// outside the lock: it asks the caller.
    pub fn flush(
        &self,
        fh: FileHandle,
        store: impl FnOnce(&str, &[u8]) -> Result<(), Errno>,
    ) -> Result<(), Errno> {
        let (path, buffer) = {
            let open = self.lock();
            match open.get(&fh.0) {
                Some(handle) if handle.dirty => (handle.path.clone(), handle.buffer.clone()),
                Some(_) => return Ok(()),
                None => return Err(Errno::EBADF),
            }
        };
        store(&path, &buffer)?;
        if let Some(handle) = self.lock().get_mut(&fh.0) {
            handle.dirty = false;
        }
        Ok(())
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
