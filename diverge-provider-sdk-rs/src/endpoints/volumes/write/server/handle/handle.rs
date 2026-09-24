//! Answering a write, from a scope and a manager.

use futures_util::StreamExt as _;

use super::super::channel_request;
use super::super::response;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::write::client::{channel_response, request};
use crate::endpoints::volumes::{names, refusal};
use crate::server::answers::Answers;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume::Volume as _;
use crate::server::volume_manager::VolumeManager;
use crate::shared::containers::write_path;
use crate::shared::error::Error;

/// Collect the content, put the file in the volume, and end the
/// scope.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::server::volume::Volume::lock) for the length of
/// the write, so no container has the volume while the file lands,
/// and none sees the half between. A volume held at all — mounted in
/// a running container, or under another request — is the write
/// refused with [`refusal::mounted`] before the content is asked for:
/// the hold never waits, and the endpoint has no frame for it but the
/// error. The hold is given back whatever the write answered.
///
/// # The content is asked for, once the volume is held
///
/// One channel, opened with the empty
/// [`channel_request::Frame`], and read as the stream of what the
/// client answers on it: every body a piece, an error the client's
/// own, a channel that ends without its finish
/// [`refusal::unfinished`], a frame that will not decode
/// [`refusal::content`]. The volume is handed the stream and puts the
/// file in place as the pieces arrive; a stream that ends in an
/// error is a write the volume abandons, and its error is the
/// answer.
///
/// # Every failure becomes a frame
///
/// A path that is not one of names, or that names no file, is
/// [`refusal::path`] or [`refusal::root`] before anything is looked
/// at; a name the caller has no volume by is [`refusal::unknown`]; a
/// manager or a volume that will not answer is its error, flattened.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
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
    let frame = match write(&scope, manager, client_identity, &request).await {
        Ok(()) => response::Frame::Written(write_path::response::Frame),
        Err(error) => response::Frame::Error(error),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The path checked, the volume found and locked, the content asked
/// for and written in, and the volume unlocked; or the one error the
/// endpoint answers with, whichever step it came from.
async fn write<M>(
    scope: &ScopeHandle,
    manager: &M,
    client_identity: &str,
    request: &request::Frame,
) -> Result<(), Error>
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
    let mut ask = Vec::new();
    channel_request::Frame
        .encode(&mut Writer::new(&mut ask))
        .unwrap_or_else(|error| match error {});
    let content = Answers::open(scope, &ask).await.map(|item| match item {
        Ok(payload) => match channel_response::Frame::decode(&payload) {
            Ok(channel_response::Frame::Body(body)) => Ok(payload.slice_ref(body.0)),
            Ok(channel_response::Frame::Error(error)) => Err(error),
            Err(error) => Err(refusal::content(&error)),
        },
        Err(_) => Err(refusal::unfinished()),
    });
    let written = volume.write(&request.path, content).await;
    volume.unlock().await;
    written.map_err(Into::into)
}
