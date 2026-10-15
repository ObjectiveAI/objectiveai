//! Answering a deletion, from a scope and a manager.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::volumes::delete::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::volumes::delete::client::request;
use diverge_sdk::provider::endpoints::volumes::refusal;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use crate::protocol::hold::Held;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume::Volume as _;
use crate::protocol::changes::Changes;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::error::Error;

/// Remove the volume and end the scope.
///
/// # The hold is answered here
///
/// A volume anything has is never deleted, and the wire has a word
/// for each: [`Mounted`](response::Frame::Mounted) for a running
/// container, [`FuseServed`](response::Frame::FuseServed) for a FUSE
/// serve, [`Held`](response::Frame::Held) for an operation in flight.
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::protocol::volume::Volume::lock); a volume held
/// at all is answered so, and the manager is never asked. A provider
/// whose capabilities keep no volumes answers
/// [`Unsupported`](response::Frame::Unsupported) first.
/// A hold that was taken is never given back on success: the volume
/// it was on is gone, and
/// [`delete`](VolumeManager::delete) is told so. On failure it is
/// given back, and the volume is as it was.
///
/// # Every failure becomes a frame
///
/// A name the caller has no volume by is [`refusal::unknown`]; a
/// manager or a volume that will not answer is its error, flattened.
/// A volume gone is told to every listing open, by name.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
    changes: Arc<Changes>,
    capabilities: &Capabilities,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    if !capabilities.volumes() {
        let mut buffer = Vec::new();
        if response::Frame::Unsupported.encode(&mut Writer::new(&mut buffer)).is_ok() {
            scope.send_response(&buffer).await;
        }
        scope.send_response_finish().await;
        return;
    }
    let frame = match delete(manager, client_identity, &request.name).await {
        Ok(()) => {
            changes.changed(client_identity, &request.name);
            response::Frame::Deleted
        }
        Err(frame) => frame,
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// `Ok` is the volume gone; `Err` is the refusal, whichever step
/// refused — who has the volume, or the error — and nothing changed.
async fn delete<M>(
    manager: &M,
    client_identity: &str,
    name: &str,
) -> Result<(), response::Frame>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = match manager.get(client_identity, name).await {
        Ok(Some(volume)) => volume,
        Ok(None) => return Err(response::Frame::Error(refusal::unknown(name))),
        Err(error) => return Err(response::Frame::Error(error.into())),
    };
    if let Err(held) = volume.lock().await {
        return Err(match held {
            Held::Mounted => response::Frame::Mounted,
            Held::FuseServed => response::Frame::FuseServed,
            Held::Locked => response::Frame::Held,
        });
    }
    match manager.delete(client_identity, name).await {
        Ok(()) => Ok(()),
        Err(error) => {
            // The volume is as it was, so the hold goes back.
            volume.unlock().await;
            Err(response::Frame::Error(error.into()))
        }
    }
}
