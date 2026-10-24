//! A lock while it is held.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::{Error, platform};

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

    /// Leave `contents` in the file, replacing whatever was there:
    /// what a holder says of itself — its process id, as the Diverge
    /// programs write — for whoever finds the file locked and reads
    /// it with [`holder`](super::holder). Written on a blocking
    /// task, through a second handle on the same open file, so the
    /// lock is untouched.
    pub async fn write(&self, contents: Vec<u8>) -> Result<(), Error> {
        let path = self.path.clone();
        let file = self.file.try_clone().map_err(|source| Error::Write {
            path: path.clone(),
            source,
        })?;
        let on_task = path.clone();
        tokio::task::spawn_blocking(move || {
            let mut file = file;
            file.set_len(0)?;
            file.write_all(&contents)?;
            file.flush()
        })
        .await
        .map_err(|source| Error::Blocking {
            path: on_task.clone(),
            source,
        })?
        .map_err(|source| Error::Write { path, source })
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
