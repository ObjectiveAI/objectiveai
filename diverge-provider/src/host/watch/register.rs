//! The watcher, armed and registered, and where it went dark.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use tokio::sync::mpsc;

/// One watch's watcher — the platform's own, behind `notify` — the
/// directories registered with it, and the directories it could not
/// watch.
///
/// # What is kept, and for how long
///
/// 1. A directory registered by itself is kept with whether its
///    registration is recursive, for as long as it exists.
/// 2. A directory that arrives — created, or renamed in — is
///    registered only when nothing registered covers it: not when it
///    is one already, and not when it lies beneath a recursive
///    registration, which the platform keeps watching new directories
///    under by itself. So a watch that never degraded registers
///    nothing after its root, however many directories come and go.
/// 3. A directory is dark at most once, however often it is tried.
/// 4. A path that leaves — removed, or renamed out — lets go of every
///    registration and every dark directory at it or beneath it, the
///    watcher told to forget each registration, so a directory that
///    arrives there again is judged afresh by rule 2.
pub struct Registered {
    watcher: RecommendedWatcher,
    /// Every directory registered by itself, and whether recursively.
    watched: HashMap<PathBuf, bool>,
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
    pub fn arm(sender: mpsc::Sender<notify::Result<notify::Event>>, lost: std::sync::Arc<tokio::sync::Notify>) -> notify::Result<Self> {
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            // On notify's own thread, which must not block: a full
            // queue loses the event and says so, and the pump walks
            // again; a receiver that is gone — the watch dropped —
            // means the event has nobody to reach.
            // An access — the walk itself opens every directory —
            // changes no tree, and is never queued: a walk's opens
            // would fill the queue and walk again, without end.
            if matches!(&result, Ok(event) if matches!(event.kind, notify::EventKind::Access(_))) {
                return;
            }
            if let Err(mpsc::error::TrySendError::Full(_)) = sender.try_send(result) {
                lost.notify_one();
            }
        })?;
        Ok(Registered {
            watcher,
            watched: HashMap::new(),
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
            self.watched.insert(dir.to_path_buf(), true);
            return Ok(());
        }
        if let Err(error) = self.watcher.watch(dir, RecursiveMode::NonRecursive) {
            // Rule 3.
            if !self.dark.iter().any(|dark| dark == dir) {
                self.dark.push(dir.to_path_buf());
            }
            return Err(error);
        }
        self.watched.insert(dir.to_path_buf(), false);
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

    /// A directory arrived at `dir`: registered by [`register`](Self::register)'s
    /// rules when nothing registered covers it (rule 2), and nothing
    /// otherwise.
    pub fn arrived(&mut self, dir: &Path) {
        let covered = self.watched.contains_key(dir)
            || dir.ancestors().skip(1).any(|ancestor| self.watched.get(ancestor).is_some_and(|recursive| *recursive));
        if !covered {
            let _ = self.register(dir);
        }
    }

    /// Something left `path`: every registration and every dark
    /// directory at it or beneath it let go of (rule 4). A path
    /// beneath a recursive registration, which never registers
    /// anything of its own, finds nothing to let go of.
    pub fn left(&mut self, path: &Path) {
        if self.watched.contains_key(path) {
            let gone: Vec<PathBuf> = self.watched.keys().filter(|watched| watched.starts_with(path)).cloned().collect();
            for watched in gone {
                self.watched.remove(&watched);
                let _ = self.watcher.unwatch(&watched);
            }
        }
        self.dark.retain(|dark| !dark.starts_with(path));
    }

    /// The dark directories, as they stand: what a walk consults to
    /// set each directory's `changes`.
    pub fn dark(&self) -> Vec<PathBuf> {
        self.dark.clone()
    }
}
