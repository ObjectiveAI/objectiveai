//! Answering an edit-capacity question, from a scope and a manager.

use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::edit_capacity::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

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
/// put a failure, so there is nothing else to say, and a name the
/// caller cannot see is the ordinary reason.
///
/// A response that will not ENCODE is the one failure with nowhere to
/// go, and the scope simply finishes without an answer. A caller reads
/// that as a provider that said nothing, which is what happened.
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
    let frame = match manager
        .edit_capacity(client_identity, &request.name)
        .await
    {
        Ok(bytes) => response::Frame::Capacity(bytes),
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
