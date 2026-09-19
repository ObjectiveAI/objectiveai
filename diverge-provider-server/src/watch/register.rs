//! The watcher, armed and registered, and where it went dark.

use std::fs;
use std::path::{Path, PathBuf};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use tokio::sync::mpsc;

/// One watch's watcher — the platform's own, behind `notify` — and
/// the directories it could not watch.
pub struct Registered {
    watcher: RecommendedWatcher,
    /// Every directory whose own registration failed. Each is dark
    /// with everything beneath it: walked, present in the tree, and
    /// reported with `changes` false, because nothing under it will
    /// ever arrive as a delta.
    dark: Vec<PathBuf>,
}

impl Registered {
    /// Make a watcher whose events queue toward `sender`.
    ///
    /// Nothing is watched yet; [`register`](Self::register) does that.
    /// Arming before the walk is what lets a change during the walk
    /// arrive as a delta after the snapshot rather than be lost
    /// between the two.
    pub fn arm(sender: mpsc::UnboundedSender<notify::Result<notify::Event>>) -> notify::Result<Self> {
        let watcher = notify::recommended_watcher(move |result| {
            // On notify's own thread; an unbounded send never blocks,
            // and a receiver that is gone — the watch dropped — means
            // the event has nobody to reach.
            let _ = sender.send(result);
        })?;
        Ok(Registered {
            watcher,
            dark: Vec::new(),
        })
    }

    /// Register `dir` and everything beneath it, resiliently.
    ///
    /// A directory is watched recursively in one registration where
    /// the platform allows it: notify walks it, watches every
    /// subdirectory, and keeps watching the ones created later.
    /// Where that fails — inotify's watch limit, most likely — the
    /// directory is watched by itself, non-recursively, and each child
    /// directory is registered by these same rules. A directory whose
    /// OWN watch fails is dark: recorded, with everything beneath it,
    /// and the error returned. A child going dark does not fail its
    /// parent — the corner is skipped and the stream goes on — so
    /// only the root's failure reaches the caller.
    pub fn register(&mut self, dir: &Path) -> notify::Result<()> {
        if self.watcher.watch(dir, RecursiveMode::Recursive).is_ok() {
            return Ok(());
        }
        if let Err(error) = self.watcher.watch(dir, RecursiveMode::NonRecursive) {
            self.dark.push(dir.to_path_buf());
            return Err(error);
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                let _ = self.register(&entry.path());
            }
        }
        Ok(())
    }

    /// The dark directories, as they stand: what a walk consults to
    /// set each directory's `changes`.
    pub fn dark(&self) -> Vec<PathBuf> {
        self.dark.clone()
    }
}
