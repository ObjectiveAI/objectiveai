//! The inotify watch, armed and registered.

use std::fs;
use std::path::Path;

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use tokio::sync::mpsc;

use super::Ignore;

/// Make a watcher whose events queue toward `sender`.
///
/// Nothing is watched yet; [`register`] does that. Arming before the
/// walk is what lets a change during the walk arrive as a delta after
/// the snapshot rather than be lost between the two.
pub fn arm(
    sender: mpsc::UnboundedSender<notify::Result<notify::Event>>,
) -> notify::Result<RecommendedWatcher> {
    notify::recommended_watcher(move |result| {
        // On notify's own thread; an unbounded send never blocks, and
        // a receiver that is gone — the connection ended — means the
        // event has nobody to reach.
        let _ = sender.send(result);
    })
}

/// Register `dir` and everything beneath it, resiliently.
///
/// The rules, in order:
///
/// - An excluded directory is not watched, and nothing under it.
/// - A directory with no excluded path beneath it is watched
///   recursively in one registration: notify walks it, watches every
///   subdirectory, and keeps watching the ones created later.
/// - Otherwise — an excluded path lies beneath it, or the recursive
///   registration failed, the inotify watch limit most likely — the
///   directory is watched by itself, non-recursively, and each child
///   directory is registered by these same rules. A child that fails
///   is skipped, its changes simply not streamed, rather than ending
///   the stream.
///
/// Only failing to watch `dir` itself is an error, and only for the
/// root does the caller treat it as one.
pub fn register(
    watcher: &mut RecommendedWatcher,
    dir: &Path,
    ignore: &Ignore,
) -> notify::Result<()> {
    if ignore.excluded(dir) {
        return Ok(());
    }
    if !ignore.contains_excluded(dir)
        && watcher.watch(dir, RecursiveMode::Recursive).is_ok()
    {
        return Ok(());
    }
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let _ = register(watcher, &entry.path(), ignore);
        }
    }
    Ok(())
}
