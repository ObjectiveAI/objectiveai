//! Answering a filetree, from a scope and a manager.

use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::filetree::client::request;
use crate::endpoints::volumes::{names, refusal};
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume::Volume as _;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Walk the volume and end the scope.
///
/// One question, one answer, done — so this consumes the scope rather
/// than handing it back.
///
/// # Under the exclusive hold
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::server::volume::Volume::lock) for the length of
/// the walk, so what is reported is the volume at rest, with no
/// container changing it. A volume held at all — mounted in a
/// running container, or under another request — is the filetree
/// refused with [`refusal::mounted`]: the hold never waits, and the
/// endpoint has no frame for it but the error. The hold is given
/// back whatever the walk answered.
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
    let frame = match filetree(manager, client_identity, &request).await {
        Ok(tree) => response::Frame::Tree(tree),
        Err(error) => response::Frame::Error(error),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The path checked, the volume found, locked, walked, and unlocked;
/// or the one error the endpoint answers with, whichever step it came
/// from.
async fn filetree<M>(
    manager: &M,
    client_identity: &str,
    request: &request::Frame,
) -> Result<Vec<crate::shared::filetree::response::Node>, Error>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    if !names::ok(&request.path) {
        return Err(refusal::path(&request.path));
    }
    let volume = manager
        .get(client_identity, &request.name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| refusal::unknown(&request.name))?;
    if !volume.lock().await {
        return Err(refusal::mounted(&request.name));
    }
    let tree = volume.filetree(&request.path).await;
    volume.unlock().await;
    tree.map_err(Into::into)
}
