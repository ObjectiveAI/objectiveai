//! Answering a creation, from a scope and a manager.

use serde_json::Value;

use super::super::response;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::create::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Make the volume and end the scope.
///
/// [`Created`](response::Frame::Created) carries nothing, which is the
/// whole answer: the name the caller chose is the name it already has,
/// so there is no identifier to hand back and nothing to describe that
/// a [`list`](crate::endpoints::volumes::list) would not describe
/// better.
///
/// # What a taken name does is the manager's
///
/// This turns a refusal into an [`Error`](response::Frame::Error) and
/// does not decide when one is owed. See
/// [`VolumeManager`] for why: the wire has one error per endpoint and
/// no vocabulary for the reasons.
pub async fn handle<M>(
    mut scope: ScopeHandle,
    client_identity: &str,
    manager: &M,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let frame = match request::Frame::decode(scope.request()) {
        Ok(request) => {
            match manager
                .create(client_identity, &request.name, request.bytes)
                .await
            {
                Ok(()) => response::Frame::Created,
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
