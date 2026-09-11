//! Answering an edit, from a scope and a manager.


use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::edit::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Resize the volume and end the scope.
///
/// The same three arguments a
/// [`create`](crate::endpoints::volumes::create) carries and a
/// different meaning: there the name is being made, here it is being
/// found.
///
/// # The manager says why not
///
/// Two refusals have their own frames:
/// [`InsufficientCapacity`](response::Frame::InsufficientCapacity)
/// when the provider cannot reserve the size, and
/// [`ContentTooLarge`](response::Frame::ContentTooLarge) when the
/// volume holds more than the size and so cannot be shrunk to it. The
/// manager is what knows either, so it answers the matching
/// [`Edit`](response::Edit) and this turns that into the frame. Any
/// other refusal is an [`Error`](response::Frame::Error), and this does
/// not decide when one is owed.
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
        .edit(client_identity, &request.name, request.bytes)
        .await
    {
        Ok(response::Edit::Edited) => response::Frame::Edited,
        Ok(response::Edit::InsufficientCapacity) => {
            response::Frame::InsufficientCapacity
        }
        Ok(response::Edit::ContentTooLarge) => response::Frame::ContentTooLarge,
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
