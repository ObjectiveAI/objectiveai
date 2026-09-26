//! Where an ephemeral serve's scratch lives, how one is made, and
//! how the leftovers of an earlier life are swept.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;

use super::sparse;
use crate::Limit;

/// The scratch directory and the cap the scratch counts against,
/// shared by every stored volume.
///
/// The directory is `<containers.podman.storage_path>/ephemeral/`:
/// the filesystem the containers' overlays are on, so that
/// `container_overlay_disk`, which [`Limit`] enforces over the
/// running containers' `disk`, is an honest cap over the serves'
/// `overlay_disk` beside them.
#[derive(Debug)]
pub struct Scratch {
    dir: PathBuf,
    disk: Arc<Limit>,
}

/// Bytes held against the cap for one serve; given back on drop, an
/// atomic and nothing else.
#[derive(Debug)]
pub struct Lease {
    disk: Arc<Limit>,
    bytes: u64,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.disk.give(self.bytes);
    }
}

impl Scratch {
    pub fn new(dir: PathBuf, disk: Arc<Limit>) -> Self {
        Scratch { dir, disk }
    }

    /// The directory made, and every leftover in it removed: what the
    /// provider does once at start, before any serve. A file that
    /// will not go is left, as a mount that will not unmount is.
    pub async fn sweep(&self) -> io::Result<()> {
        fs::create_dir_all(&self.dir).await?;
        let mut entries = fs::read_dir(&self.dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let _ = fs::remove_file(entry.path()).await;
        }
        Ok(())
    }

    /// `overlay_disk` taken from the cap, or `None` with nothing
    /// taken.
    pub fn lease(&self, overlay_disk: u64) -> Option<Lease> {
        self.disk.take(overlay_disk).then(|| Lease {
            disk: Arc::clone(&self.disk),
            bytes: overlay_disk,
        })
    }

    /// A fresh scratch file, handed over as a std handle for the
    /// blocking closure that will own it: made under a v4 UUID name,
    /// marked sparse, and made to vanish when its handle closes — on
    /// Unix unlinked at once, the inode living with the handle; on
    /// Windows opened to delete on close — so its removal is the
    /// host's and nothing here removes it on drop.
    pub async fn create(&self) -> io::Result<std::fs::File> {
        let path = self.dir.join(uuid::Uuid::new_v4().to_string());
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(windows)]
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_DELETE_ON_CLOSE);
        let file = options.open(&path).await?;
        sparse::sparse(&file).await?;
        #[cfg(not(windows))]
        fs::remove_file(&path).await?;
        Ok(file.into_std().await)
    }
}
