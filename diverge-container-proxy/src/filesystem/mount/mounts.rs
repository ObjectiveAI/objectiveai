//! The mounts made so far, held for the proxy's life.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    pub fn holds(&self, path: &Path) -> bool {
        self.lock().iter().any(|(held, _)| held == path)
    }

    /// Keep a mount just made; `Err` is a path already held, and the
    /// mount handed in is dropped — unmounted — with the refusal.
    pub fn insert(&self, path: PathBuf, mounted: Mounted) -> Result<(), String> {
        let mut held = self.lock();
        if held.iter().any(|(existing, _)| *existing == path) {
            return Err(format!("{} is already a mount", path.display()));
        }
        held.push((path, mounted));
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(PathBuf, Mounted)>> {
        self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
