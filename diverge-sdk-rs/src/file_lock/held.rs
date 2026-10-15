//! A lock while it is held.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::platform;

/// An exclusive lock on one file, held until this is dropped.
///
/// Dropping it unlocks the file and closes it, in that order; a
/// process that dies holding one is unlocked by the kernel the same
/// way. The file stays on disk. There is no `release` method: letting
/// go is dropping, and nothing else, so a lock cannot be forgotten
/// open.
#[derive(Debug)]
pub struct Held {
    /// The lock file, as the caller named it.
    path: PathBuf,
    /// The open file whose lock this is. Held for the lifetime of the
    /// guard, since closing it would let the lock go.
    file: File,
}

impl Held {
    /// Pair a locked file with its path.
    pub(super) fn new(path: PathBuf, file: File) -> Self {
        Held { path, file }
    }

    /// The lock file, as the caller named it.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Held {
    /// Unlock, then let the file close. An unlock that fails is not
    /// reported — there is nobody to report it to from a drop — and
    /// the close that follows lets the lock go regardless.
    fn drop(&mut self) {
        let _ = platform::unlock(&self.file);
    }
}
