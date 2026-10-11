//! Answering a deletion, from a scope and a manager.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::images::delete::client::request;
use diverge_sdk::provider::endpoints::images::delete::server::response;
use diverge_sdk::provider::endpoints::images::refusal;
use crate::protocol::changes::Changes;
use crate::protocol::image_manager::ImageManager;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Remove the image and end the scope.
///
/// The manager says what happened:
/// [`Deleted`](response::Deletion::Deleted) is the image gone, told to
/// every listing open by name; [`InUse`](response::Deletion::InUse)
/// is a running container of the caller running it, and the wire's
/// own word for that; a name the caller has no image by is
/// [`refusal::unknown`]; a manager that will not answer is its error,
/// flattened. The request arrives decoded, as every handler's does.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
    changes: Arc<Changes>,
) where
    M: ImageManager,
    M::Error: Into<Error>,
{
    let frame = match manager.delete(client_identity, &request.name).await {
        Ok(Some(response::Deletion::Deleted)) => {
            changes.changed(client_identity, &request.name);
            response::Frame::Deleted
        }
        Ok(Some(response::Deletion::InUse)) => response::Frame::InUse,
        Ok(None) => response::Frame::Error(refusal::unknown(&request.name)),
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
