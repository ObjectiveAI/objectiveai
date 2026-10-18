//! A tree scope: the container's filesystem, watched from `/` and
//! streamed until the server says stop.

use std::path::PathBuf;
use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::filesystem::tree::client::request;
use diverge_sdk::container_proxy::outside::endpoints::filesystem::tree::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree;

use crate::encode::encoded;
use crate::filesystem::tree;

/// The tree's root.
const ROOT: &str = "/";

/// Serve one tree until the server stops it or the proxy ends.
///
/// The tree is [`tree::stream`]'s, rooted at `/` and leaving out what
/// the request names: the snapshot, then every change, each sent as
/// the scope's response. Any channel the server opens on the scope is
/// the stop, which has no answer: the scope is finished, and the
/// stream drops with this task, which unregisters everything the
/// watch held.
///
/// What is an error, per the wire: the watcher could not be made,
/// the root could not be watched at all, or a blocking task died —
/// each answered with one `Error` carrying the reason, then the
/// finish. Lost events are not: the stream re-walks and sends a
/// fresh snapshot on the same scope.
pub async fn tree(scope: ScopeHandle, frame: request::Frame) {
    let ignore = Arc::new(tree::Ignore::new(frame.ignore));
    let mut frames = tree::stream(PathBuf::from(ROOT), ignore);
    loop {
        tokio::select! {
            next = frames.recv() => match next {
                Some(Ok(frame)) => send(&scope, frame).await,
                Some(Err(failure)) => {
                    error(&scope, failure.kind, &failure.reason).await;
                    return;
                }
                None => break,
            },
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
