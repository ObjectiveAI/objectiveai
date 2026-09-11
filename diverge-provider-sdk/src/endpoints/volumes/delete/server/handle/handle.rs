//! Answering a deletion, from a scope and a manager.


use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::delete::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Remove the volume and end the scope.
///
/// # The manager says whether it was mounted
///
/// A volume [`mounted`](crate::server::mount::Mount) into a running
/// container is never deleted, and the wire has a word for it:
/// [`Mounted`](response::Frame::Mounted). The manager is what knows
/// whether the volume is mounted, so it answers
/// [`Deletion::Mounted`](response::Deletion::Mounted) and this turns
/// that into the frame. Nothing here checks anything itself.
///
/// A volume under somebody's live
/// [`watch`](crate::endpoints::volumes::watch) is not mounted, and
/// what the manager does about one is the manager's — the protocol
/// does not adjudicate it, so neither does this.
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
    let frame = match manager.delete(client_identity, &request.name).await {
        Ok(response::Deletion::Deleted) => response::Frame::Deleted,
        Ok(response::Deletion::Mounted) => response::Frame::Mounted,
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
