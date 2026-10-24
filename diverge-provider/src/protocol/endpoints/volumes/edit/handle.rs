//! Answering an edit, from a scope and a manager.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::volumes::edit::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::volumes::edit::client::request;
use diverge_sdk::provider::endpoints::volumes::refusal;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use crate::protocol::hold::Held;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume::Volume as _;
use crate::protocol::changes::Changes;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::error::Error;

/// Change the volume and end the scope.
///
/// A name and a change: there a [`create`](diverge_sdk::provider::endpoints::volumes::create)
/// makes the name, here it is found. A change made is told to every
/// listing open, by name; a change refused is told to nobody.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::protocol::volume::Volume::lock) for the length of
/// the change, so no container has it while its size or its mode
/// changes — which is what makes the mode changeable at all, since
/// a container is bound under one mode for its life. A volume held
/// at all is the edit refused, the volume as it was, with the
/// endpoint's own word for who has it —
/// [`Mounted`](response::Frame::Mounted),
/// [`FuseServed`](response::Frame::FuseServed) or
/// [`Held`](response::Frame::Held) — and the hold never waits. The
/// hold is given back whatever the change answered. A provider whose
/// capabilities keep no volumes answers
/// [`Unsupported`](response::Frame::Unsupported) first, and one that
/// keeps no volumes in the mode a change names answers
/// [`ModeUnsupported`](response::Frame::ModeUnsupported) before the
/// hold is asked.
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
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
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
    let named = match request.change {
        request::Change::Bytes(_) => None,
        request::Change::Mode(mode) | request::Change::Both { mode, .. } => Some(mode),
    };
    let unsupported = match named {
        Some(mode) => capabilities.volume(mode).is_none(),
        None => false,
    };
    let frame = match edit(manager, client_identity, &request.name, request.change, unsupported).await {
        Ok(response::Edit::Edited) => {
            changes.changed(client_identity, &request.name);
            response::Frame::Edited
        }
        Ok(response::Edit::InsufficientCapacity) => {
            response::Frame::InsufficientCapacity
        }
        Ok(response::Edit::ContentTooLarge) => response::Frame::ContentTooLarge,
        Err(frame) => frame,
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The volume found, locked, changed, and unlocked; or the one
/// refusal the endpoint answers with, whichever step it came from.
/// `mode_unsupported` is the handler's finding that the change names
/// a mode the capabilities keep no volumes in, answered once the
/// volume is known to be the caller's and before the hold is asked.
async fn edit<M>(
    manager: &M,
    client_identity: &Identity,
    name: &str,
    change: request::Change,
    mode_unsupported: bool,
) -> Result<response::Edit, response::Frame>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = match manager.get(client_identity, name).await {
        Ok(Some(volume)) => volume,
        Ok(None) => return Err(response::Frame::Error(refusal::unknown(name))),
        Err(error) => return Err(response::Frame::Error(error.into())),
    };
    if mode_unsupported {
        return Err(response::Frame::ModeUnsupported);
    }
    if let Err(held) = volume.lock().await {
        return Err(match held {
            Held::Mounted => response::Frame::Mounted,
            Held::FuseServed => response::Frame::FuseServed,
            Held::Locked => response::Frame::Held,
        });
    }
    let edit = volume.edit(change).await;
    volume.unlock().await;
    edit.map_err(|error| response::Frame::Error(error.into()))
}
