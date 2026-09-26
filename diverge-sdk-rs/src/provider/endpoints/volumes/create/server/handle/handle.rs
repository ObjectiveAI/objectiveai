//! Answering a creation, from a scope and a manager.


use super::super::response;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::volumes::create::client::request;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Make the volume and end the scope.
///
/// [`Created`](response::Frame::Created) carries nothing, which is the
/// whole answer: the name the caller chose is the name it already has,
/// so there is no identifier to hand back and nothing to describe that
/// a [`list`](crate::provider::endpoints::volumes::list) would not describe
/// better.
///
/// # The manager says whether there was room
///
/// A size the provider cannot reserve is
/// [`InsufficientCapacity`](response::Frame::InsufficientCapacity),
/// and the manager is what knows it, so it answers
/// [`Creation::InsufficientCapacity`](response::Creation::InsufficientCapacity)
/// and this turns that into the frame.
///
/// # What a taken name does is the manager's
///
/// This turns any other refusal into an
/// [`Error`](response::Frame::Error) and does not decide when one is
/// owed. See [`VolumeManager`] for why: the wire has one error per
/// endpoint and no vocabulary for the reasons.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
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
        .create(client_identity, &request.name, request.bytes, request.mode)
        .await
    {
        Ok(response::Creation::Created) => response::Frame::Created,
        Ok(response::Creation::InsufficientCapacity) => {
            response::Frame::InsufficientCapacity
        }
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
