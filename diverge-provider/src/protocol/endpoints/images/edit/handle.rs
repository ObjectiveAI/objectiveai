//! Answering an edit, from a scope and a manager.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use crate::protocol::changes::Changes;
use crate::protocol::image_manager::ImageManager;
use diverge_sdk::provider::endpoints::images::edit::client::request;
use diverge_sdk::provider::endpoints::images::edit::server::response;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Set or clear the image's description and end the scope.
///
/// The rules:
///
/// 1. A provider whose capabilities keep no Store answers
///    [`Unsupported`](response::Frame::Unsupported) and asks nothing.
/// 2. The manager sets the description of the caller's image under the
///    digest — or clears it, for a request with none — and answers
///    [`Edited`](response::Frame::Edited).
/// 3. Every listing open is told, by digest, only when the
///    description is other than it was: an edit to the same value is
///    `Edited` and tells nothing.
/// 4. A digest the caller owns no image by is
///    [`NotFound`](response::Frame::NotFound); a manager that will not
///    answer is its error, flattened.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
    manager: &M,
    changes: Arc<Changes>,
    capabilities: &Capabilities,
) where
    M: ImageManager,
    M::Error: Into<Error>,
{
    let frame = if !capabilities.images_store {
        response::Frame::Unsupported
    } else {
        match manager.edit(client_identity, &request.digest, request.description.as_ref()).await {
            Ok(Some(changed)) => {
                if changed {
                    changes.changed(client_identity, &request.digest);
                }
                response::Frame::Edited
            }
            Ok(None) => response::Frame::NotFound,
            Err(error) => response::Frame::Error(error.into()),
        }
    };
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
