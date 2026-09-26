//! Answering a read, from a scope and a manager.

use std::pin::pin;

use futures_util::StreamExt as _;

use super::super::response;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::volumes::read::client::request;
use crate::provider::endpoints::volumes::{names, refusal};
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::volume::Volume as _;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::containers::read;
use crate::shared::error::Error;

/// Stream the file out of the volume and end the scope.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::provider::server::volume::Volume::lock) for the length of
/// the read, so what is streamed is the file at rest, with no
/// container writing to it. A volume held at all — mounted in a
/// running container, or under another request — is the read refused
/// with [`refusal::mounted`]: the hold never waits, and the endpoint
/// has no frame for it but the error. The hold is given back after
/// the finish, whatever the read answered.
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
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let (volume, stream) = match open(manager, client_identity, &request).await {
        Ok(opened) => opened,
        Err(error) => {
            send(&scope, &response::Frame::Error(error)).await;
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
/// for reading; or the one error the endpoint answers with, whichever
/// step it came from — with the hold given back where it was taken.
async fn open<M>(
    manager: &M,
    client_identity: &str,
    request: &request::Frame,
) -> Result<(M::Volume, <M::Volume as crate::provider::server::volume::Volume>::Read), Error>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    if !names::ok(&request.path) {
        return Err(refusal::path(&request.path));
    }
    if request.path.is_empty() {
        return Err(refusal::root());
    }
    let volume = manager
        .get(client_identity, &request.name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| refusal::unknown(&request.name))?;
    if !volume.lock().await {
        return Err(refusal::mounted(&request.name));
    }
    match volume.read(&request.path).await {
        Ok(stream) => Ok((volume, stream)),
        Err(error) => {
            volume.unlock().await;
            Err(error.into())
        }
    }
}
