//! Answering a filetree, from a scope and a manager.

use diverge_sdk::provider::endpoints::volumes::filetree::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::volumes::filetree::client::request;
use diverge_sdk::provider::endpoints::volumes::{names, refusal};
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use crate::protocol::hold::Held;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume::Volume as _;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::error::Error;

/// Walk the volume and end the scope.
///
/// One question, one answer, done — so this consumes the scope rather
/// than handing it back.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::protocol::volume::Volume::lock) for the length of
/// the walk, so what is reported is the volume at rest, with no
/// container changing it. A volume held at all is the filetree
/// refused with the endpoint's own word for who has it —
/// [`Mounted`](response::Frame::Mounted),
/// [`FuseServed`](response::Frame::FuseServed) or
/// [`Held`](response::Frame::Held) — and the hold never waits. The
/// hold is given back whatever the walk answered. A provider whose
/// capabilities keep no volumes answers
/// [`Unsupported`](response::Frame::Unsupported) first.
///
/// # Every failure becomes a frame
///
/// A path that is not one of names is [`refusal::path`] before
/// anything is looked at; a name the caller has no volume by is
/// [`refusal::unknown`]; a manager or a volume that will not answer
/// is its error, flattened.
///
/// A response that will not ENCODE is the one failure with nowhere to
/// go, and the scope simply finishes without an answer.
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
    let frame = match filetree(manager, client_identity, &request).await {
        Ok(tree) => response::Frame::Tree(tree),
        Err(frame) => frame,
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The path checked, the volume found, locked, walked, and unlocked;
/// or the one refusal the endpoint answers with, whichever step it
/// came from.
async fn filetree<M>(
    manager: &M,
    client_identity: &str,
    request: &request::Frame,
) -> Result<Vec<diverge_sdk::shared::filetree::response::Node>, response::Frame>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    if !names::ok(&request.path) {
        return Err(response::Frame::Error(refusal::path(&request.path)));
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
    let tree = volume.filetree(&request.path).await;
    volume.unlock().await;
    tree.map_err(|error| response::Frame::Error(error.into()))
}
