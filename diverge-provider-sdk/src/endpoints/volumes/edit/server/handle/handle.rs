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
/// # Shrinking below what is used is not decided here
///
/// A manager that refuses gets an [`Error`](response::Frame::Error) and
/// one that allows it gets an [`Edited`](response::Frame::Edited).
/// Nothing in this protocol promises `bytes` is ever at least
/// `bytes_used`, and this does not start.
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
        Ok(()) => response::Frame::Edited,
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
