//! Answering a serve, from a scope and a manager.

use std::sync::Arc;

use tokio::task::JoinSet;

use super::super::response;
use crate::container_proxy_endpoints::fuse::mount::client::execute::Ask;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::refusal;
use crate::endpoints::volumes::serve::client::{channel_request, request};
use crate::frame::client::ClientFrame;
use crate::server::scope_handle::ScopeHandle;
use crate::server::served::Served;
use crate::server::volume::Volume as _;
use crate::server::volume_manager::VolumeManager;
use crate::shared::containers::fuse;
use crate::shared::containers::fuse::ack::Refused;
use crate::shared::error::Error;

/// Hold the volume, answer every ask, and end the scope at the stop.
///
/// # Under the shared hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`mounted`](crate::server::volume::Volume::mount) — the shared
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
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
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
    let served = match volume.serve(request.overlay_disk).await {
        Ok(served) => Arc::new(served),
        Err(error) => {
            volume.unmount().await;
            return refuse(scope, error.into()).await;
        }
    };
    send(&scope, &response::Frame::Serving).await;

    let scope = Arc::new(scope);
    let mut tasks = JoinSet::new();
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let ask = match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Ask(ask)) => Ask::from(ask),
            Ok(channel_request::Frame::Stop) => break,
            Err(_) => {
                scope.send_channel_response_finish(channel).await;
                continue;
            }
        };
        tasks.spawn(one(Arc::clone(&scope), Arc::clone(&served), channel, ask));
    }
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
/// then the finish.
async fn one<V: Served>(scope: Arc<ScopeHandle>, served: Arc<V>, channel: u32, ask: Ask) {
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
        Ask::Write { path, offset, bytes } => ack(&mut buffer, served.write(&path, offset, bytes).await),
        Ask::Truncate { path, size } => ack(&mut buffer, served.truncate(&path, size).await),
        Ask::Setattr { path, attrs } => ack(&mut buffer, served.setattr(&path, attrs).await),
        Ask::Remove { path } => ack(&mut buffer, served.remove(&path).await),
        Ask::Rename { from, to } => ack(&mut buffer, served.rename(&from, &to).await),
        Ask::Mkdir { path } => ack(&mut buffer, served.mkdir(&path).await),
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
