//! The watch: opened, and the stream it is.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use diverge_sdk::shared::filetree::response::Frame;
use futures_util::Stream;
use tokio::sync::{Mutex, Notify, mpsc};
use tokio::task::JoinHandle;

use super::{Error, Mapped, Registered, map, paths, walk};

/// Watch the directory at `root`: the snapshot first, then one frame
/// per change, until the stream is dropped.
///
/// The root is made canonical and must be a directory. The watcher is
/// armed, the root registered and the tree walked, all in one
/// blocking task, in that order; the snapshot is the stream's first
/// item. A pump task then takes every event the watcher reports, maps
/// it under a blocking task of its own — one at a time, so the
/// frames' order is the events' order — and sends its frames; a lost
/// event re-walks and sends a fresh snapshot. Bounded both ways: at
/// most [`EVENTS`] events wait to be mapped — one more is lost, and
/// re-walks — and one frame waits to be read, the pump waiting on its
/// reader. The watcher lives in
/// the pump, and dropping the stream aborts the pump, which
/// unregisters everything the watcher held.
pub async fn watch(root: &Path) -> Result<Watch, Error> {
    let root = paths::canonical(root).await.map_err(Error::Io)?;
    if !tokio::fs::metadata(&root).await.is_ok_and(|meta| meta.is_dir()) {
        return Err(Error::Root(root));
    }
    let (events_sender, events) = mpsc::channel(EVENTS);
    let lost = Arc::new(Notify::new());
    let armed = tokio::task::spawn_blocking({
        let root = root.clone();
        let lost = Arc::clone(&lost);
        move || {
            let mut registered = Registered::arm(events_sender, lost)?;
            registered.register(&root)?;
            let dark = registered.dark();
            let children = walk::children(&root, &root, &dark);
            Ok::<_, notify::Error>((registered, children))
        }
    })
    .await;
    let (registered, children) = match armed {
        Ok(Ok(armed)) => armed,
        Ok(Err(error)) => return Err(Error::Watch(error)),
        Err(error) => return Err(Error::Walk(error)),
    };
    let (frames_sender, frames) = mpsc::channel(1);
    let _ = frames_sender.try_send(Ok(Frame::Snapshot { children }));
    let pump = tokio::spawn(pump(root, Arc::new(Mutex::new(registered)), events, lost, frames_sender));
    Ok(Watch { frames, pump })
}

/// Every event mapped and its frames sent, until the watcher's
/// thread is gone — the stream's last item an error — or the stream
/// is gone, which ends this quietly.
async fn pump(
    root: PathBuf,
    registered: Arc<Mutex<Registered>>,
    mut events: mpsc::Receiver<notify::Result<notify::Event>>,
    lost: Arc<Notify>,
    frames: mpsc::Sender<Result<Frame, Error>>,
) {
    loop {
        let mapped = tokio::select! {
            result = events.recv() => {
                let Some(result) = result else { break };
                match result {
                    Ok(event) => {
                        let root = root.clone();
                        let registered = Arc::clone(&registered);
                        match tokio::task::spawn_blocking(move || map(event, &root, &registered)).await {
                            Ok(mapped) => mapped,
                            Err(error) => {
                                let _ = frames.send(Err(Error::Walk(error))).await;
                                return;
                            }
                        }
                    }
                    Err(_) => Mapped::Resync,
                }
            }
            // Events were lost to a full queue: walk again.
            () = lost.notified() => Mapped::Resync,
        };
        let list = match mapped {
            Mapped::Frames(list) => list,
            Mapped::Resync => {
                let root = root.clone();
                let registered = Arc::clone(&registered);
                let walked = tokio::task::spawn_blocking(move || {
                    let dark = registered.blocking_lock().dark();
                    walk::children(&root, &root, &dark)
                })
                .await;
                match walked {
                    Ok(children) => vec![Frame::Snapshot { children }],
                    Err(error) => {
                        let _ = frames.send(Err(Error::Walk(error))).await;
                        return;
                    }
                }
            }
        };
        for frame in list {
            if frames.send(Ok(frame)).await.is_err() {
                return;
            }
        }
    }
    let _ = frames.send(Err(Error::Stopped)).await;
}

/// How many events wait to be mapped before the next is lost.
pub const EVENTS: usize = 1024;

/// One directory's watch as a stream: the snapshot, then every
/// change, then — only if the watch died — the error, and the end.
///
/// Dropping it ends the watch: the pump is aborted, and the watcher
/// in it dropped, which unregisters everything it held.
#[must_use = "a watch that is not polled reports nothing"]
pub struct Watch {
    frames: mpsc::Receiver<Result<Frame, Error>>,
    pump: JoinHandle<()>,
}

impl Stream for Watch {
    type Item = Result<Frame, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.frames.poll_recv(cx)
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.pump.abort();
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}
