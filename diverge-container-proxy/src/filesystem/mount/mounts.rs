//! The mounts made so far, held for the proxy's life.

use std::path::{Path, PathBuf};

use tokio::sync::Mutex;

use super::Mounted;

/// Every mount `/fuse/mount` has made, by path, kept so it stays
/// mounted: dropping a [`Mounted`] unmounts, and nothing here drops
/// one. A second mount at a path already held is refused.
pub struct Mounts {
    held: Mutex<Vec<(PathBuf, Mounted)>>,
}

impl Mounts {
    pub fn new() -> Self {
        Mounts {
            held: Mutex::new(Vec::new()),
        }
    }

    /// Whether a mount is held at exactly `path`.
    pub async fn holds(&self, path: &Path) -> bool {
        self.held.lock().await.iter().any(|(held, _)| held == path)
    }

    /// Keep a mount just made; `Err` is a path already held, and the
    /// mount handed in is dropped — unmounted — with the refusal.
    pub async fn insert(&self, path: PathBuf, mounted: Mounted) -> Result<(), String> {
        let mut held = self.held.lock().await;
        if held.iter().any(|(existing, _)| *existing == path) {
            return Err(format!("{} is already a mount", path.display()));
        }
        held.push((path, mounted));
        Ok(())
    }
}
