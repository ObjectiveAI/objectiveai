//! Answering an edit, from a scope and a manager.

use super::super::response;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::volumes::edit::client::request;
use crate::provider::endpoints::volumes::refusal;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::volume::Volume as _;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Change the volume and end the scope.
///
/// A name and a change: there a [`create`](crate::provider::endpoints::volumes::create)
/// makes the name, here it is found.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::provider::server::volume::Volume::lock) for the length of
/// the change, so no container has it while its size or its mode
/// changes — which is what makes the mode changeable at all, since
/// a container is bound under one mode for its life. A volume held
/// at all — mounted in a running container, or under another
/// request — is the edit refused with [`refusal::mounted`] and the
/// volume as it was: the hold never waits, and the endpoint has no
/// frame for it but the error. The hold is given back whatever the
/// change answered.
///
/// # The volume says why not
///
/// Two refusals have their own frames:
/// [`InsufficientCapacity`](response::Frame::InsufficientCapacity)
/// when the provider cannot reserve the size, and
/// [`ContentTooLarge`](response::Frame::ContentTooLarge) when the
/// volume holds more than the size and so cannot be shrunk to it. The
/// volume is what knows either, so it answers the matching
/// [`Edit`](response::Edit) and this turns that into the frame. A
/// name the caller has no volume by is [`refusal::unknown`]; any
/// other refusal is the provider's error, flattened.
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
    let frame = match edit(manager, client_identity, &request.name, request.change).await {
        Ok(response::Edit::Edited) => response::Frame::Edited,
        Ok(response::Edit::InsufficientCapacity) => {
            response::Frame::InsufficientCapacity
        }
        Ok(response::Edit::ContentTooLarge) => response::Frame::ContentTooLarge,
        Err(error) => response::Frame::Error(error),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The volume found, locked, changed, and unlocked; or the one error
/// the endpoint answers with, whichever step it came from.
async fn edit<M>(
    manager: &M,
    client_identity: &str,
    name: &str,
    change: request::Change,
) -> Result<response::Edit, Error>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = manager
        .get(client_identity, name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| refusal::unknown(name))?;
    if !volume.lock().await {
        return Err(refusal::mounted(name));
    }
    let edit = volume.edit(change).await;
    volume.unlock().await;
    edit.map_err(Into::into)
}
