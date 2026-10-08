//! Answering a listing, and keeping it answered, from a scope, a
//! manager and the word of changes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use super::super::response;
use crate::provider::endpoints::volumes::list::client::channel_request;
use crate::provider::server::volume_changes::VolumeChanges;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::client::ClientFrame;
use crate::wire::server::scope_handle::ScopeHandle;

/// Send the caller every volume it has, the word that the listing is
/// whole, and every change after, until the caller stops.
///
/// In order:
///
/// 1. The changes subscribed to, before anything is read, so nothing
///    that happens meanwhile is missed; then the manager asked which
///    volumes the caller has. A manager that will not answer is the
///    [`Error`](response::Frame::Error), and the scope finishes.
/// 2. Every volume listed sent as [`Added`](response::Frame::Added),
///    then exactly one [`Listed`](response::Frame::Listed) — at once,
///    with nothing before it, for a caller with no volume.
/// 3. For the scope's life: a change to one of the caller's volumes,
///    heard by name, is the manager asked again and the difference
///    told — a name not listed before as `Added`, a name listed whose
///    `bytes` or `mode` differ as [`Changed`](response::Frame::Changed),
///    a name listed and gone as [`Removed`](response::Frame::Removed)
///    with the volume as it was. A change to another caller's volume
///    is nothing. A subscription that fell behind is the manager
///    asked again the same way. A manager that will not answer then
///    is the `Error`, last.
/// 4. The finish, when the caller's [`Stop`](channel_request::Frame::Stop)
///    arrives, the caller is gone, or the error was sent.
///
/// # `client_identity` is an argument
///
/// Because this crate does not know it. It belongs to the CONNECTION,
/// which a provider authenticated and this half was handed already
/// open, so whatever dispatches to this is what knows whose scope it
/// is. See [`Mount`](crate::provider::server::mount::Mount) for why a volume
/// namespace needs one at all.
///
/// # A response that will not encode
///
/// Is the one failure with nowhere to go, and is not sent; the
/// listing goes on without it.
pub async fn handle<M>(scope: ScopeHandle, client_identity: &str, manager: &M, changes: Arc<VolumeChanges>)
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let mut nudges = changes.subscribe();
    let mut known: HashMap<String, response::Volume> = HashMap::new();
    match manager.list(client_identity).await {
        Ok(volumes) => {
            for volume in volumes {
                send(&scope, &response::Frame::Added(volume.clone())).await;
                known.insert(volume.name.clone(), volume);
            }
            send(&scope, &response::Frame::Listed).await;
        }
        Err(error) => {
            send(&scope, &response::Frame::Error(error.into())).await;
            scope.send_response_finish().await;
            return;
        }
    }
    loop {
        tokio::select! {
            nudge = nudges.recv() => {
                let again = match nudge {
                    Ok(changed) => changed.identity.as_ref() == client_identity,
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                if again {
                    match manager.list(client_identity).await {
                        Ok(volumes) => tell(&scope, &mut known, volumes).await,
                        Err(error) => {
                            send(&scope, &response::Frame::Error(error.into())).await;
                            break;
                        }
                    }
                }
            }
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Stop) => break,
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
        }
    }
    scope.send_response_finish().await;
}

/// The listing as the manager has it now against the listing as
/// sent: every difference told, and the sent listing brought up to
/// date.
async fn tell(scope: &ScopeHandle, known: &mut HashMap<String, response::Volume>, now: Vec<response::Volume>) {
    let mut seen: HashMap<String, response::Volume> = HashMap::with_capacity(now.len());
    for volume in now {
        match known.get(&volume.name) {
            None => send(scope, &response::Frame::Added(volume.clone())).await,
            Some(was) if was.bytes != volume.bytes || was.mode != volume.mode => {
                send(scope, &response::Frame::Changed(volume.clone())).await;
            }
            Some(_) => {}
        }
        seen.insert(volume.name.clone(), volume);
    }
    for (name, was) in known.drain() {
        if !seen.contains_key(&name) {
            send(scope, &response::Frame::Removed(was)).await;
        }
    }
    *known = seen;
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
