//! Answering a deletion, from a scope and a manager.

use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::delete::client::request;
use crate::endpoints::volumes::refusal;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume::Volume as _;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Remove the volume and end the scope.
///
/// # Mounted is the lock, and it is answered here
///
/// A volume [`mounted`](crate::server::mount::Mount) into a running
/// container is never deleted, and the wire has a word for it:
/// [`Mounted`](response::Frame::Mounted). The volume is
/// [`got`](VolumeManager::get) and then
/// [`locked`](crate::server::volume::Volume::lock); a lock that is
/// held — a running container has the volume, or a stat or an edit
/// is in flight on it — is `Mounted`, and the manager is never asked.
/// A lock that was taken is never given back on success: the volume
/// it was on is gone, and
/// [`delete`](VolumeManager::delete) is told so. On failure it is
/// given back, and the volume is as it was.
///
/// A volume under somebody's live
/// [`watch`](crate::endpoints::volumes::watch) is not mounted and
/// holds no lock, and what the manager does about one is the
/// manager's — the protocol does not adjudicate it, so neither does
/// this.
///
/// # Every failure becomes a frame
///
/// A name the caller has no volume by is [`refusal::unknown`]; a
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
    let frame = match delete(manager, client_identity, &request.name).await {
        Ok(true) => response::Frame::Deleted,
        Ok(false) => response::Frame::Mounted,
        Err(error) => response::Frame::Error(error),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// `true` is the volume gone; `false` is its lock held, and nothing
/// changed; the error is whichever step refused.
async fn delete<M>(
    manager: &M,
    client_identity: &str,
    name: &str,
) -> Result<bool, Error>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = manager
        .get(client_identity, name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| refusal::unknown(name))?;
    if !volume.lock().await.map_err(Into::into)? {
        return Ok(false);
    }
    match manager.delete(client_identity, name).await {
        Ok(()) => Ok(true),
        Err(error) => {
            // The volume is as it was, so the lock goes back; a
            // failure to give it back is reported after the failure
            // that caused it, which is the one the caller acts on.
            let _ = volume.unlock().await;
            Err(error.into())
        }
    }
}
