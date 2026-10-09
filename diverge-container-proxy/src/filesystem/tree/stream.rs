//! A tree as a stream: armed, walked, and every change after.

use std::path::PathBuf;
use std::sync::Arc;

use diverge_sdk::shared::filetree;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use super::{Ignore, Mapped, Watch, map, walk};

/// Why a stream stopped short: the watch could not be made, the root
/// could not be watched, a walk died, a map died.
pub struct Failure {
    /// Which step: `watch`, `walk` or `map`.
    pub kind: &'static str,
    /// The reason, for a reader.
    pub reason: String,
}

/// The tree at `root`, as a stream of frames: the snapshot first,
/// then every change under it, until the receiver is dropped or the
/// watch fails, which is the one error and the end.
///
/// The watcher is armed BEFORE the walk, so a change during the walk
/// waits in the events queue and goes out as a delta after the
/// snapshot — replayed onto a tree that may already show it, which
/// the fold tolerates — rather than falling between the two. Arming,
/// registering and walking are one blocking task; every event is
/// mapped in another, one at a time, so the stream's frames are the
/// events' order. A corner the watch could not cover is still walked,
/// its directory's `changes` false. Lost events — the queue
/// overflowed, or notify reported an error — re-walk and send a fresh
/// snapshot. Every path a frame carries is components from `root`.
///
/// The receiver dropped is the end: the task notices, stops, and the
/// watch drops with it, which unregisters everything it held.
pub fn stream(root: PathBuf, ignore: Arc<Ignore>) -> mpsc::UnboundedReceiver<Result<filetree::response::Frame, Failure>> {
    let (out, frames) = mpsc::unbounded_channel();
    tokio::spawn(run(root, ignore, out));
    frames
}

/// The stream's own task.
async fn run(root: PathBuf, ignore: Arc<Ignore>, out: mpsc::UnboundedSender<Result<filetree::response::Frame, Failure>>) {
    let root = Arc::new(root);
    let (sender, mut events) = mpsc::unbounded_channel();

    let armed = tokio::task::spawn_blocking({
        let ignore = Arc::clone(&ignore);
        let root = Arc::clone(&root);
        move || {
            let mut watch = Watch::arm(sender)?;
            watch.register(&root, &ignore)?;
            let dark = watch.dark();
            let children = walk::children(&root, &ignore, &dark);
            Ok::<_, notify::Error>((watch, children))
        }
    })
    .await;
    let (watch, children) = match armed {
        Ok(Ok(armed)) => armed,
        Ok(Err(reason)) => {
            let _ = out.send(Err(failure("watch", &reason.to_string())));
            return;
        }
        Err(reason) => {
            let _ = out.send(Err(failure("walk", &reason.to_string())));
            return;
        }
    };
    let watch = Arc::new(Mutex::new(watch));

    if out.send(Ok(filetree::response::Frame::Snapshot { children })).is_err() {
        return;
    }

    loop {
        tokio::select! {
            result = events.recv() => {
                let Some(result) = result else {
                    break;
                };
                let mapped = match result {
                    Ok(event) => {
                        let ignore = Arc::clone(&ignore);
                        let watch = Arc::clone(&watch);
                        let root = Arc::clone(&root);
                        match tokio::task::spawn_blocking(move || map(event, &ignore, &watch, &root)).await {
                            Ok(mapped) => mapped,
                            Err(reason) => {
                                let _ = out.send(Err(failure("map", &reason.to_string())));
                                return;
                            }
                        }
                    }
                    Err(_) => Mapped::Resync,
                };
                let frames = match mapped {
                    Mapped::Frames(frames) => frames,
                    Mapped::Resync => {
                        let ignore = Arc::clone(&ignore);
                        let watch = Arc::clone(&watch);
                        let root = Arc::clone(&root);
                        let children = tokio::task::spawn_blocking(move || {
                            let dark = watch.blocking_lock().dark();
                            walk::children(&root, &ignore, &dark)
                        })
                        .await;
                        match children {
                            Ok(children) => vec![filetree::response::Frame::Snapshot { children }],
                            Err(reason) => {
                                let _ = out.send(Err(failure("walk", &reason.to_string())));
                                return;
                            }
                        }
                    }
                };
                for frame in frames {
                    if out.send(Ok(frame)).is_err() {
                        return;
                    }
                }
            }
            // Nobody is reading: the stream is over, and the watch
            // with it.
            () = out.closed() => break,
        }
    }
}

/// One failure, named and explained.
fn failure(kind: &'static str, reason: &str) -> Failure {
    Failure {
        kind,
        reason: reason.to_string(),
    }
}
