//! Answering a listing, from a scope and a manager.


use super::super::response;
use crate::wire::encode::{Encode, Writer};
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Answer a listing and end the scope.
///
/// One question, one answer, done — so this consumes the scope rather
/// than handing it back. There is nothing a provider does with a
/// listing's scope afterwards.
///
/// # `client_identity` is an argument
///
/// Because this crate does not know it. It belongs to the CONNECTION,
/// which a provider authenticated and this half was handed already
/// open, so whatever dispatches to this is what knows whose scope it
/// is. See [`Mount`](crate::provider::server::mount::Mount) for why a volume
/// namespace needs one at all.
///
/// # Every failure becomes a frame
///
/// A manager that will not answer is an
/// [`Error`](response::Frame::Error) — the endpoint has one place to
/// put a failure, so there is nothing else to say. A malformed request
/// never arrives:
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request to dispatch it, and this one carried nothing to begin
/// with.
///
/// A response that will not ENCODE is the one failure with nowhere to
/// go, and the scope simply finishes without an answer. A caller reads
/// that as a provider that said nothing, which is what happened.
pub async fn handle<M>(
    scope: ScopeHandle,
    client_identity: &str,
    manager: &M,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let frame = match manager.list(client_identity).await {
        Ok(volumes) => response::Frame::Volumes(volumes),
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
