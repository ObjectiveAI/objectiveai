//! Answering a create-capacity question, from a scope and a manager.

use diverge_sdk::provider::endpoints::volumes::create_capacity::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::error::Error;

/// Answer the question and end the scope.
///
/// One question, one answer, done — so this consumes the scope rather
/// than handing it back. There is nothing a provider does with the
/// scope afterwards.
///
/// # Every failure becomes a frame
///
/// A manager that will not answer is an
/// [`Error`](response::Frame::Error) — the endpoint has one place to
/// put a failure, so there is nothing else to say. A malformed request
/// never arrives:
/// [`server::handle`](crate::protocol::handle::handle) reads every
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
    let frame = match manager.create_capacity(client_identity).await {
        Ok(bytes) => response::Frame::Capacity(bytes),
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
