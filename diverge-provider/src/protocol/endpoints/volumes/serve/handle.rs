//! Answering a serve, from a scope and a manager.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::task::JoinSet;

use diverge_sdk::provider::endpoints::volumes::serve::server::channel_response::filetree;
use diverge_sdk::provider::endpoints::volumes::serve::server::response;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute::Ask;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::volumes::refusal;
use diverge_sdk::provider::endpoints::volumes::serve::client::{channel_request, request};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::served::Served;
use crate::protocol::volume::Volume as _;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::containers::fuse;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::response::VolumeMode;
use diverge_sdk::shared::error::Error;
use diverge_sdk::shared::filetree::response::{Frame as Tree, Node};

/// How many changes a filetree channel may fall behind by before it
/// is sent a fresh snapshot instead of the changes it missed.
const CHANGES_BEHIND: usize = 256;

/// Hold the volume, answer every ask, and end the scope at the stop.
///
/// # Under the shared hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`mounted`](crate::protocol::volume::Volume::mount) — the shared
/// hold a run takes — for the scope's life, so nothing examines,
/// resizes or removes it meanwhile and other serves and runs may hold
/// it beside this one. A volume held exclusively is the serve refused
/// with [`refusal::mounted`]. The hold is given back after the finish.
///
/// # Every ask is its own task
///
/// Each channel the caller opens is decoded and answered on a task of
/// its own through the volume's [`Served`], so asks in flight at once
/// are answered in whatever order the volume answers them; a channel
/// request that will not decode is finished with nothing. The stop
/// ends the loop; the tasks still running are awaited, the scope is
/// finished, and the volume unmounted — in that order, so the finish
/// follows the last answer. A caller that leaves without a stop ends
/// the loop the same way.
///
/// # The tree is kept here
///
/// A filetree channel is answered from the same [`Served`]: a walk of
/// it by [`list`](Served::list) and [`stat`](Served::stat) for the
/// snapshot, and then every change this serve's own asks make, which
/// the task answering a mutation reports once the volume answered it
/// `ok`. The changes travel on a broadcast every filetree channel
/// subscribes to; a channel that falls behind by more changes than
/// the broadcast keeps is sent a fresh snapshot, as a source that lost
/// track of the tree does. The
/// broadcast is dropped at the stop, which is what ends every
/// filetree channel with the scope.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle<M>(scope: ScopeHandle, request: request::Frame, client_identity: &str, manager: &M)
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = match manager.get(client_identity, &request.name).await {
        Ok(Some(volume)) => volume,
        Ok(None) => return refuse(scope, refusal::unknown(&request.name)).await,
        Err(error) => return refuse(scope, error.into()).await,
    };
    if !volume.mount().await {
        return refuse(scope, refusal::mounted(&request.name)).await;
    }
    let mode = volume.mode().await;
    if mode != request.mode {
        volume.unmount().await;
        send(
            &scope,
            &response::Frame::VolumeMode(VolumeMode {
                name: request.name.clone(),
                mode,
            }),
        )
        .await;
        scope.send_response_finish().await;
        return;
    }
    let served = match volume.serve(request.overlay_disk).await {
        Ok(served) => Arc::new(served),
        Err(error) => {
            volume.unmount().await;
            return refuse(scope, error.into()).await;
        }
    };
    send(&scope, &response::Frame::Serving).await;

    let scope = Arc::new(scope);
    let (changes, _) = broadcast::channel(CHANGES_BEHIND);
    let mut tasks = JoinSet::new();
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Ask(ask)) => {
                tasks.spawn(one(Arc::clone(&scope), Arc::clone(&served), channel, Ask::from(ask), changes.clone()));
            }
            Ok(channel_request::Frame::Filetree) => {
                tasks.spawn(watch(Arc::clone(&scope), Arc::clone(&served), channel, changes.subscribe()));
            }
            Ok(channel_request::Frame::Stop) => break,
            Err(_) => {
                scope.send_channel_response_finish(channel).await;
                continue;
            }
        }
    }
    drop(changes);
    while tasks.join_next().await.is_some() {}
    scope.send_response_finish().await;
    volume.unmount().await;
}

/// The one response a refused serve gets, then the finish.
async fn refuse(scope: ScopeHandle, error: Error) {
    send(&scope, &response::Frame::Error(error)).await;
    scope.send_response_finish().await;
}

/// One response on channel `0`, encoded and sent; one that will not
/// encode is not sent.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}

/// One ask answered from the volume: its one frame on its channel,
/// then the finish; and, for a mutation the volume answered `ok`, the
/// change it made, reported to every filetree channel.
async fn one<V: Served>(scope: Arc<ScopeHandle>, served: Arc<V>, channel: u32, ask: Ask, changes: broadcast::Sender<Tree>) {
    let mut buffer = Vec::new();
    let encoded = match ask {
        Ask::Stat { path } => {
            let answer = served.stat(&path).await;
            let frame = match &answer {
                Ok(Some(stat)) => fuse::stat::response::Frame::Present(*stat),
                Ok(None) => fuse::stat::response::Frame::Missing,
                Err(message) => fuse::stat::response::Frame::Error(message),
            };
            frame.encode(&mut Writer::new(&mut buffer)).is_ok()
        }
        Ask::Read { path, offset, length } => {
            let answer = served.read(&path, offset, length).await;
            let frame = match &answer {
                Ok(Some(bytes)) => fuse::read::response::Frame::Present(bytes),
                Ok(None) => fuse::read::response::Frame::Missing,
                Err(message) => fuse::read::response::Frame::Error(message),
            };
            frame.encode(&mut Writer::new(&mut buffer)).is_ok()
        }
        Ask::List { path } => {
            let answer = served.list(&path).await;
            let frame = match &answer {
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
            };
            frame.encode(&mut Writer::new(&mut buffer)).is_ok()
        }
        Ask::Write { path, offset, bytes } => {
            let existed = existed(&*served, &changes, &path).await;
            let result = served.write(&path, offset, bytes).await;
            if result.is_ok() {
                report(&*served, &changes, &path, existed).await;
            }
            ack(&mut buffer, result)
        }
        Ask::Truncate { path, size } => {
            let existed = existed(&*served, &changes, &path).await;
            let result = served.truncate(&path, size).await;
            if result.is_ok() {
                report(&*served, &changes, &path, existed).await;
            }
            ack(&mut buffer, result)
        }
        Ask::Setattr { path, attrs } => {
            let result = served.setattr(&path, attrs).await;
            if result.is_ok() {
                report(&*served, &changes, &path, true).await;
            }
            ack(&mut buffer, result)
        }
        Ask::Remove { path } => {
            let result = served.remove(&path).await;
            if result.is_ok() {
                let _ = changes.send(Tree::Removed { path: components(&path) });
            }
            ack(&mut buffer, result)
        }
        Ask::Rename { from, to } => {
            let result = served.rename(&from, &to).await;
            if result.is_ok() {
                let _ = changes.send(Tree::Removed { path: components(&from) });
                report(&*served, &changes, &to, false).await;
            }
            ack(&mut buffer, result)
        }
        Ask::Mkdir { path } => {
            let result = served.mkdir(&path).await;
            if result.is_ok() {
                report(&*served, &changes, &path, false).await;
            }
            ack(&mut buffer, result)
        }
    };
    if encoded {
        scope.send_channel_response(channel, &buffer).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// The one-frame answer every mutation shares, encoded into `buffer`.
fn ack(buffer: &mut Vec<u8>, result: Result<(), Refused>) -> bool {
    let frame = match &result {
        Ok(()) => fuse::ack::Frame::Ok,
        Err(Refused::ReadOnly) => fuse::ack::Frame::ReadOnly,
        Err(Refused::Error(message)) => fuse::ack::Frame::Error(message),
    };
    frame.encode(&mut Writer::new(buffer)).is_ok()
}

/// Whether something is at the path before a mutation that may make
/// it, so that the change can be told as an insertion or a
/// modification; asked only while a filetree channel is listening,
/// since nothing else wants to know.
async fn existed<V: Served>(served: &V, changes: &broadcast::Sender<Tree>, path: &str) -> bool {
    changes.receiver_count() > 0 && matches!(served.stat(path).await, Ok(Some(_)))
}

/// The node at the path as it now is, reported whole — a directory
/// with everything beneath it — as an insertion or a modification;
/// nothing at the path is reported as a removal. Nothing is walked
/// while no filetree channel is listening.
async fn report<V: Served>(served: &V, changes: &broadcast::Sender<Tree>, path: &str, existed: bool) {
    if changes.receiver_count() == 0 {
        return;
    }
    let frame = match node(served, path).await {
        Some(node) if existed => Tree::Modified { path: components(path), node },
        Some(node) => Tree::Inserted { path: components(path), node },
        None => Tree::Removed { path: components(path) },
    };
    let _ = changes.send(frame);
}

/// One filetree channel: the snapshot, then every change until the
/// scope ends, then the finish. A channel that fell behind is sent a
/// fresh snapshot in place of the changes it missed.
async fn watch<V: Served>(scope: Arc<ScopeHandle>, served: Arc<V>, channel: u32, mut changes: broadcast::Receiver<Tree>) {
    send_tree(&scope, channel, &Tree::Snapshot { children: walk(&*served, "").await }).await;
    loop {
        match changes.recv().await {
            Ok(frame) => send_tree(&scope, channel, &frame).await,
            Err(broadcast::error::RecvError::Lagged(_)) => {
                send_tree(&scope, channel, &Tree::Snapshot { children: walk(&*served, "").await }).await;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// One filetree frame on its channel; one that will not encode is not
/// sent.
async fn send_tree(scope: &ScopeHandle, channel: u32, frame: &Tree) {
    let mut buffer = Vec::new();
    if filetree::Frame::Filetree(frame.clone()).encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_channel_response(channel, &buffer).await;
    }
}

/// The entries of the directory at `path`, each with everything
/// beneath it, as the serve answers a list and a stat of each; an
/// entry the serve could not describe is a file of unknown size.
fn walk<'a, V: Served>(served: &'a V, path: &'a str) -> Pin<Box<dyn Future<Output = Vec<Node>> + Send + 'a>> {
    Box::pin(async move {
        let Ok(Some(listed)) = served.list(path).await else {
            return Vec::new();
        };
        let mut nodes = Vec::with_capacity(listed.len());
        for entry in listed {
            let child = join(path, &entry.name);
            let stat = served.stat(&child).await.ok().flatten();
            nodes.push(match entry.kind {
                fuse::Kind::Directory => Node::Directory {
                    name: entry.name,
                    created_at: None,
                    modified_at: stat.map(|stat| stat.mtime.secs),
                    changes: true,
                    children: walk(served, &child).await,
                },
                fuse::Kind::File => Node::File {
                    name: entry.name,
                    size: stat.map(|stat| stat.size),
                    created_at: None,
                    modified_at: stat.map(|stat| stat.mtime.secs),
                },
            });
        }
        nodes
    })
}

/// The node at `path`, whole, or `None` for nothing there.
async fn node<V: Served>(served: &V, path: &str) -> Option<Node> {
    let stat = served.stat(path).await.ok().flatten()?;
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    Some(match stat.kind {
        fuse::Kind::Directory => Node::Directory {
            name,
            created_at: None,
            modified_at: Some(stat.mtime.secs),
            changes: true,
            children: walk(served, path).await,
        },
        fuse::Kind::File => Node::File {
            name,
            size: Some(stat.size),
            created_at: None,
            modified_at: Some(stat.mtime.secs),
        },
    })
}

/// A path's components, as a filetree frame carries them: none for
/// the root.
fn components(path: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').map(str::to_string).collect()
    }
}

/// A child's path under `path`, `/`-separated as the asks spell it.
fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}/{name}")
    }
}
