//! Answering a deletion, from a scope and a manager.


use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::delete::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Remove the volume and end the scope.
///
/// # It does not check who is using it
///
/// A volume may be [`mounted`](crate::server::mount::Mount) into a
/// running container or under somebody's live
/// [`watch`](crate::endpoints::volumes::watch), and this asks the
/// manager either way. Whether that is refused, performed, or performed
/// and left to break what was holding it is the manager's to decide —
/// the protocol does not adjudicate it, so neither does this.
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
        Ok(()) => response::Frame::Deleted,
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
