//! Answering an edit, from a scope and a manager.

use serde_json::Value;

use super::super::response;
use crate::decode::Decode;
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
pub async fn handle<M>(
    manager: &M,
    client_identity: &str,
    mut scope: ScopeHandle,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let frame = match request::Frame::decode(scope.request()) {
        Ok(request) => {
            match manager
                .edit(client_identity, &request.name, request.bytes)
                .await
            {
                Ok(()) => response::Frame::Edited,
                Err(error) => response::Frame::Error(error.into()),
            }
        }
        Err(error) => {
            response::Frame::Error(Error(Value::String(error.to_string())))
        }
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
