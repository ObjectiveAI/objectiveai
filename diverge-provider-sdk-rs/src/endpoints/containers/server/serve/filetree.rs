//! The container's tree, watched: a scope on the proxy and a watch
//! of every volume the provider watches itself, relayed as one tree.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _};
use tokio::sync::Notify;

use super::super::family::Family;
use super::super::render;
use super::super::run::Run;
use super::super::watched::Watch;
use crate::container_proxy_endpoints::filesystem::tree::client::execute as tree;
use crate::shared::error::Error;
use crate::shared::filetree::response::{Frame, Node};

/// Open a `filesystem::tree` scope on the proxy, leaving out every
/// FUSE mount and every mount the provider watches itself, and open
/// that watch for each of those; put every frame of all of them on
/// the caller's channel as one tree; the first error last, then the
/// finish. The proxy's snapshot goes first, then each watched mount
/// planted into it, then every change as it comes. A watch that
/// could not be opened, or that died, ends the whole channel with its
/// error — a tree with a hole in it would be a tree that lied — and
/// the proxy's finish ends it without one. The proxy scope is
/// registered with the run, which stops it when the run shuts down;
/// the volume watches end with the run's tasks.
pub(crate) async fn filetree<F: Family>(run: Arc<Run>, channel: u32) {
    let opened = future::join(
        tree::execute(&run.proxy, run.ignore.clone()),
        future::join_all(run.watched.iter().map(|watched| watched.watcher.watch())),
    )
    .await;
    let ((mut frames, handle), watches) = match opened {
        (Err(error), _) => {
            run.respond(channel, F::filetree_error(&render::proxy(error))).await;
            run.finish(channel).await;
            return;
        }
        (Ok(proxy), watches) => {
            let mut opened = Vec::with_capacity(watches.len());
            for watch in watches {
                match watch {
                    Ok(watch) => opened.push(watch),
                    Err(error) => {
                        run.respond(channel, F::filetree_error(&error)).await;
                        run.finish(channel).await;
                        return;
                    }
                }
            }
            (proxy, opened)
        }
    };
    run.trees.lock().await.insert(handle.scope(), handle.clone());
    let ending = Arc::new(Ending {
        run: Arc::clone(&run),
        channel,
        done: AtomicBool::new(false),
        stop: Notify::new(),
    });

    // The container's snapshot first, then each watched mount planted
    // into it, so a change under a mount never precedes the mount.
    let mut planted = Vec::with_capacity(watches.len());
    match frames.next().await {
        Some(Ok(frame)) => run.respond(channel, F::filetree(frame)).await,
        Some(Err(error)) => {
            ending.fail::<F>(&proxy_error(error)).await;
        }
        None => ending.end::<F>().await,
    }
    for (watched, mut watch) in run.watched.iter().zip(watches) {
        if ending.over() {
            break;
        }
        match watch.next().await {
            Some(Ok(frame)) => {
                run.respond(channel, F::filetree(rerooted(&watched.container_path, frame))).await;
                planted.push((watched.container_path.clone(), watch));
            }
            Some(Err(error)) => ending.fail::<F>(&error).await,
            None => ending.end::<F>().await,
        }
    }
    for (container_path, watch) in planted {
        let ending = Arc::clone(&ending);
        run.spawn(async move { relay_watch::<F>(ending, container_path, watch).await }).await;
    }

    while !ending.over() {
        match next(&mut frames, &ending.stop).await {
            Some(Some(Ok(frame))) => run.respond(channel, F::filetree(frame)).await,
            Some(Some(Err(error))) => ending.fail::<F>(&proxy_error(error)).await,
            Some(None) => ending.end::<F>().await,
            None => {}
        }
    }
    run.trees.lock().await.remove(&handle.scope());
}

/// One volume watch relayed after its snapshot, every frame re-rooted
/// at the mount, until the channel is over.
async fn relay_watch<F: Family>(ending: Arc<Ending>, container_path: Vec<String>, mut watch: Watch) {
    while !ending.over() {
        match next(&mut watch, &ending.stop).await {
            Some(Some(Ok(frame))) => {
                ending
                    .run
                    .respond(ending.channel, F::filetree(rerooted(&container_path, frame)))
                    .await
            }
            Some(Some(Err(error))) => ending.fail::<F>(&error).await,
            Some(None) => ending.end::<F>().await,
            None => {}
        }
    }
}

/// The next item of `stream`, or `None` when `stop` was notified
/// first.
async fn next<S: Stream + Unpin>(stream: &mut S, stop: &Notify) -> Option<Option<S::Item>> {
    let item = std::pin::pin!(stream.next());
    let stopped = std::pin::pin!(stop.notified());
    match future::select(item, stopped).await {
        Either::Left((item, _)) => Some(item),
        Either::Right(((), _)) => None,
    }
}

/// How one filetree channel ends, once, from whichever stream ends
/// it: the error, if there was one, the finish, and every other
/// relay told to stop.
struct Ending {
    run: Arc<Run>,
    channel: u32,
    done: AtomicBool,
    stop: Notify,
}

impl Ending {
    /// Whether the channel is over.
    fn over(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// The channel ended with `error`: the error, then the finish.
    async fn fail<F: Family>(&self, error: &Error) {
        if self.done.swap(true, Ordering::AcqRel) {
            return;
        }
        self.run.respond(self.channel, F::filetree_error(error)).await;
        self.run.finish(self.channel).await;
        self.stop.notify_waiters();
    }

    /// The channel ended: the finish.
    async fn end<F: Family>(&self) {
        if self.done.swap(true, Ordering::AcqRel) {
            return;
        }
        self.run.finish(self.channel).await;
        self.stop.notify_waiters();
    }
}

/// A frame of a volume's watch as a frame of the container's tree:
/// every path with the mount's path in front of it, and a snapshot
/// as the mount point planted whole, since the proxy's tree left the
/// mount out and has no node for it. The mount point's own times are
/// not known here and are reported as none.
fn rerooted(container_path: &[String], frame: Frame) -> Frame {
    let prefixed = |path: Vec<String>| {
        let mut whole = container_path.to_vec();
        whole.extend(path);
        whole
    };
    match frame {
        Frame::Snapshot { children } => Frame::Inserted {
            path: container_path.to_vec(),
            node: Node::Directory {
                name: container_path.last().cloned().unwrap_or_default(),
                created_at: None,
                modified_at: None,
                changes: true,
                children,
            },
        },
        Frame::Inserted { path, node } => Frame::Inserted {
            path: prefixed(path),
            node,
        },
        Frame::Modified { path, node } => Frame::Modified {
            path: prefixed(path),
            node,
        },
        Frame::Removed { path } => Frame::Removed { path: prefixed(path) },
    }
}

/// The proxy's stream failing, in the wire's one error shape: its
/// own words where it refused, this end's rendering otherwise.
fn proxy_error(error: tree::ExecuteStreamError) -> Error {
    match error {
        tree::ExecuteStreamError::Refused(error) => error,
        error => render::proxy(error),
    }
}
