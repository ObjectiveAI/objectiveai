//! Answering a read, from a scope and a manager.

use diverge_sdk::shared::Identity;
use std::pin::pin;

use futures_util::StreamExt as _;

use diverge_sdk::provider::endpoints::volumes::read::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::volumes::read::client::request;
use diverge_sdk::provider::endpoints::volumes::{names, refusal};
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use crate::protocol::hold::Held;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume::Volume as _;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::containers::read;
use diverge_sdk::shared::error::Error;

/// Stream the file out of the volume and end the scope.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::protocol::volume::Volume::lock) for the length of
/// the read, so what is streamed is the file at rest, with no
/// container writing to it. A volume held at all is the read refused
/// with the endpoint's own word for who has it —
/// [`Mounted`](response::Frame::Mounted),
/// [`FuseServed`](response::Frame::FuseServed) or
/// [`Held`](response::Frame::Held) — and the hold never waits. The
/// hold is given back after the finish, whatever the read answered.
/// A provider whose capabilities keep no volumes answers
/// [`Unsupported`](response::Frame::Unsupported) first.
///
/// # Every failure becomes a frame
///
/// A path that is not one of names, or that names no file, is
/// [`refusal::path`] or [`refusal::root`] before anything is looked
/// at; a name the caller has no volume by is [`refusal::unknown`]; a
/// manager or a volume that will not answer is its error, flattened
/// — before the first piece, or after the pieces it managed, since a
/// file can fail to read halfway. The error is the last response.
///
/// A response that will not ENCODE is the one failure with nowhere to
/// go, and the scope simply finishes. A caller reads that as a
/// provider that stopped, which is what happened.
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
    client_identity: &Identity,
    manager: &M,
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
    let (volume, stream) = match open(manager, client_identity, &request).await {
        Ok(opened) => opened,
        Err(frame) => {
            send(&scope, &frame).await;
            scope.send_response_finish().await;
            return;
        }
    };
    let mut stream = pin!(stream);
    while let Some(piece) = stream.next().await {
        match piece {
            Ok(bytes) => {
                send(&scope, &response::Frame::Body(read::response::Frame(&bytes))).await;
            }
            Err(error) => {
                send(&scope, &response::Frame::Error(error.into())).await;
                break;
            }
        }
    }
    scope.send_response_finish().await;
    volume.unlock().await;
}

/// One response, encoded and sent; one that will not encode is not
/// sent.
async fn send(scope: &ScopeHandle, frame: &response::Frame<'_>) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}

/// The path checked, the volume found, locked, and its file opened
/// for reading; or the one refusal the endpoint answers with,
/// whichever step it came from — with the hold given back where it
/// was taken.
async fn open<M>(
    manager: &M,
    client_identity: &Identity,
    request: &request::Frame,
) -> Result<(M::Volume, <M::Volume as crate::protocol::volume::Volume>::Read), response::Frame<'static>>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    if !names::ok(&request.path) {
        return Err(response::Frame::Error(refusal::path(&request.path)));
    }
    if request.path.is_empty() {
        return Err(response::Frame::Error(refusal::root()));
    }
    let volume = match manager.get(client_identity, &request.name).await {
        Ok(Some(volume)) => volume,
        Ok(None) => return Err(response::Frame::Error(refusal::unknown(&request.name))),
        Err(error) => return Err(response::Frame::Error(error.into())),
    };
    if let Err(held) = volume.lock().await {
        return Err(match held {
            Held::Mounted => response::Frame::Mounted,
            Held::FuseServed => response::Frame::FuseServed,
            Held::Locked => response::Frame::Held,
        });
    }
    match volume.read(&request.path).await {
        Ok(stream) => Ok((volume, stream)),
        Err(error) => {
            volume.unlock().await;
            Err(response::Frame::Error(error.into()))
        }
    }
}
