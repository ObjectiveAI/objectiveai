//! A serve scope: one subtree of the container, answered ask by ask
//! and watched, until the server says stop.

use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::filesystem::serve::client::{channel_request, request};
use diverge_sdk::container_proxy::outside::endpoints::filesystem::serve::server::channel_response::filetree;
use diverge_sdk::container_proxy::outside::endpoints::filesystem::serve::server::response;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute::Ask;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::fuse;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::error::Error;
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::encode::encoded;
use crate::filesystem::served::Served;
use crate::filesystem::tree;
use crate::paths;
use crate::reply::reply;

/// Serve one subtree until the server stops it or the proxy ends.
///
/// The rules, in order:
///
/// 1. The path must name a directory of the container
///    ([`paths::directory`]: the root, or components each a name),
///    not under `/proc`, `/sys` or `/dev`, and a directory on disk —
///    else `Error`, then the finish.
/// 2. `Serving`.
/// 3. Every channel the server opens on the scope, answered on a
///    task of its own: an ask is answered from the subtree by
///    [`Served`] with that ask's own frame, then the finish; a
///    filetree is the subtree as [`tree::stream`] streams it — the
///    snapshot, then every change under it, the serve's own and the
///    program's alike — until the scope ends, then the finish. A
///    channel request that will not decode is finished with nothing.
/// 4. The stop ends the loop, and so does the server's connection
///    ending: every filetree task is told the scope is over, every
///    task is awaited — so the finish follows the last answer — and
///    the scope is finished.
pub async fn served(scope: ScopeHandle, frame: request::Frame) {
    let Some(root) = paths::directory(&frame.path) else {
        error(&scope, "path", "path names no directory").await;
        return;
    };
    let served = Served::new(root.clone());
    if tree::Ignore::new(Vec::new()).excluded(&root) {
        error(&scope, "path", "the path is one the proxy leaves out").await;
        return;
    }
    match tokio::fs::metadata(&root).await {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            error(&scope, "path", "not a directory").await;
            return;
        }
        Err(reason) => {
            error(&scope, "open", &reason.to_string()).await;
            return;
        }
    }
    if let Some(payload) = encoded(&response::Frame::Serving) {
        scope.send_response(&payload).await;
    }

    let scope = Arc::new(scope);
    let served = Arc::new(served);
    let root = Arc::new(root);
    let (over, _) = watch::channel(false);
    let mut tasks = JoinSet::new();
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Ask(ask)) => {
                tasks.spawn(answer(Arc::clone(&scope), Arc::clone(&served), channel, Ask::from(ask)));
            }
            Ok(channel_request::Frame::Filetree) => {
                tasks.spawn(stream(Arc::clone(&scope), Arc::clone(&root), channel, over.subscribe()));
            }
            Ok(channel_request::Frame::Stop) => break,
            Err(_) => reply(&scope, channel, None).await,
        }
    }
    over.send_replace(true);
    while tasks.join_next().await.is_some() {}
    scope.send_response_finish().await;
}

/// One ask answered from the subtree: its one frame on its channel,
/// then the finish.
async fn answer(scope: Arc<ScopeHandle>, served: Arc<Served>, channel: u32, ask: Ask) {
    let payload = match ask {
        Ask::Stat { path } => {
            let answer = served.stat(&path).await;
            encoded(&match &answer {
                Ok(Some(stat)) => fuse::stat::response::Frame::Present(*stat),
                Ok(None) => fuse::stat::response::Frame::Missing,
                Err(message) => fuse::stat::response::Frame::Error(message),
            })
        }
        Ask::Read { path, offset, length } => {
            let answer = served.read(&path, offset, length).await;
            encoded(&match &answer {
                Ok(Some(bytes)) => fuse::read::response::Frame::Present(bytes),
                Ok(None) => fuse::read::response::Frame::Missing,
                Err(message) => fuse::read::response::Frame::Error(message),
            })
        }
        Ask::List { path } => {
            let answer = served.list(&path).await;
            encoded(&match &answer {
                Ok(Some(listed)) => fuse::list::response::Frame::Entries(
                    listed
                        .iter()
                        .map(|entry| fuse::Entry {
                            name: &entry.name,
                            kind: entry.kind,
                        })
                        .collect(),
                ),
                Ok(None) => fuse::list::response::Frame::Missing,
                Err(message) => fuse::list::response::Frame::Error(message),
            })
        }
        Ask::Write { path, offset, bytes } => ack(served.write(&path, offset, &bytes).await),
        Ask::Truncate { path, size } => ack(served.truncate(&path, size).await),
        Ask::Setattr { path, attrs } => ack(served.setattr(&path, attrs).await),
        Ask::Remove { path } => ack(served.remove(&path).await),
        Ask::Rename { from, to } => ack(served.rename(&from, &to).await),
        Ask::Mkdir { path } => ack(served.mkdir(&path).await),
    };
    reply(&scope, channel, payload).await;
}

/// The one-frame answer every mutation shares.
fn ack(result: Result<(), Refused>) -> Option<Vec<u8>> {
    encoded(&match &result {
        Ok(()) => fuse::ack::Frame::Ok,
        Err(Refused::ReadOnly) => fuse::ack::Frame::ReadOnly,
        Err(Refused::Error(message)) => fuse::ack::Frame::Error(message),
    })
}

/// One filetree channel: the subtree's stream, frame by frame, until
/// the scope is over or the stream fails, then the finish.
async fn stream(scope: Arc<ScopeHandle>, root: Arc<std::path::PathBuf>, channel: u32, mut over: watch::Receiver<bool>) {
    let ignore = Arc::new(tree::Ignore::new(Vec::new()));
    let mut frames = tree::stream(root.as_ref().clone(), ignore);
    loop {
        tokio::select! {
            next = frames.recv() => match next {
                Some(Ok(frame)) => {
                    if let Some(payload) = encoded(&filetree::Frame::Filetree(frame)) {
                        scope.send_channel_response(channel, &payload).await;
                    }
                }
                Some(Err(failure)) => {
                    let error = Error(serde_json::json!({
                        "kind": failure.kind,
                        "error": failure.reason,
                    }));
                    if let Some(payload) = encoded(&filetree::Frame::Error(error)) {
                        scope.send_channel_response(channel, &payload).await;
                    }
                    break;
                }
                None => break,
            },
            changed = over.changed() => {
                if changed.is_err() || *over.borrow_and_update() {
                    break;
                }
            }
        }
    }
    scope.send_channel_response_finish(channel).await;
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
