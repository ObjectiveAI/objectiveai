//! A tree scope: the container's filesystem, watched from `/` and
//! streamed until the server says stop.

use std::sync::{Arc, Mutex};

use diverge_sdk::container_proxy::outside::endpoints::filesystem::tree::client::request;
use diverge_sdk::container_proxy::outside::endpoints::filesystem::tree::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree;
use tokio::sync::mpsc;

use crate::encode::encoded;
use crate::filesystem::tree::{self, Mapped};

/// The tree's root.
const ROOT: &str = "/";

/// Serve one tree until the server stops it or the proxy ends.
///
/// The watcher is armed BEFORE the walk, so a change during the walk
/// waits in the events queue and goes out as a delta after the
/// snapshot — replayed onto a tree that may already show it, which
/// the fold tolerates — rather than falling between the two. Arming,
/// registering and walking are one blocking task; every event is
/// mapped in another, one at a time, so the stream's frames are the
/// events' order. Any channel the server opens on the scope is the
/// stop, which has no answer: the scope is finished, and the watch
/// drops with this task, which unregisters everything it held. A
/// corner the watch could not cover is still walked, its directory's
/// `changes` false.
///
/// What is an error, per the wire: the watcher could not be made,
/// the root could not be watched at all, or a blocking task died —
/// each answered with one `Error` carrying the reason, then the
/// finish. Lost events — the queue overflowed, or notify reported an
/// error — are not: they re-walk and send a fresh snapshot on the
/// same scope.
pub async fn tree(scope: ScopeHandle, frame: request::Frame) {
    let ignore = Arc::new(tree::Ignore::new(frame.ignore));
    let (sender, mut events) = mpsc::unbounded_channel();

    let armed = tokio::task::spawn_blocking({
        let ignore = Arc::clone(&ignore);
        move || {
            let mut watch = tree::Watch::arm(sender)?;
            watch.register(std::path::Path::new(ROOT), &ignore)?;
            let dark = watch.dark();
            let children = tree::children(std::path::Path::new(ROOT), &ignore, &dark);
            Ok::<_, notify::Error>((watch, children))
        }
    })
    .await;
    let (watch, children) = match armed {
        Ok(Ok(armed)) => armed,
        Ok(Err(reason)) => {
            error(&scope, "watch", &reason.to_string()).await;
            return;
        }
        Err(reason) => {
            error(&scope, "walk", &reason.to_string()).await;
            return;
        }
    };
    let watch = Arc::new(Mutex::new(watch));

    send(&scope, filetree::response::Frame::Snapshot { children }).await;

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
                        match tokio::task::spawn_blocking(move || tree::map(event, &ignore, &watch)).await {
                            Ok(mapped) => mapped,
                            Err(reason) => {
                                error(&scope, "map", &reason.to_string()).await;
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
                        let children = tokio::task::spawn_blocking(move || {
                            let dark = watch.lock().map(|watch| watch.dark()).unwrap_or_default();
                            tree::children(std::path::Path::new(ROOT), &ignore, &dark)
                        })
                        .await;
                        match children {
                            Ok(children) => vec![filetree::response::Frame::Snapshot { children }],
                            Err(reason) => {
                                error(&scope, "walk", &reason.to_string()).await;
                                return;
                            }
                        }
                    }
                };
                for frame in frames {
                    send(&scope, frame).await;
                }
            }
            // The stop: the one thing the server says to a running
            // tree, and it has no answer but the scope's end.
            stop = scope.recv_channel_request() => {
                let _ = stop;
                break;
            }
        }
    }
    scope.send_response_finish().await;
}

/// One filetree frame, as the scope's response.
async fn send(scope: &ScopeHandle, frame: filetree::response::Frame) {
    if let Some(payload) = encoded(&response::Frame::Filetree(frame)) {
        scope.send_response(&payload).await;
    }
}

/// The error, then the finish.
async fn error(scope: &ScopeHandle, kind: &str, reason: &str) {
    let error = Error(serde_json::json!({
        "kind": kind,
        "error": reason,
    }));
    if let Some(payload) = encoded(&response::Frame::Error(error)) {
        scope.send_response(&payload).await;
    }
    scope.send_response_finish().await;
}
