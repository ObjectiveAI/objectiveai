//! Answering a stat, from a scope and a manager.

use diverge_sdk::shared::Identity;
use diverge_sdk::provider::endpoints::images::refusal;
use diverge_sdk::provider::endpoints::images::stat::client::request;
use diverge_sdk::provider::endpoints::images::stat::server::response;
use crate::protocol::image_manager::ImageManager;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Examine the image and end the scope.
///
/// One question, one answer, done. A provider whose capabilities keep
/// no image store answers [`Unsupported`](response::Frame::Unsupported)
/// and asks nothing. A name the caller has no image by
/// is [`refusal::unknown`]; a manager that will not answer is its
/// error, flattened. A response that will not encode is the one
/// failure with nowhere to go, and the scope finishes without an
/// answer. The request arrives decoded, as every handler's does.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
    manager: &M,
    capabilities: &Capabilities,
) where
    M: ImageManager,
    M::Error: Into<Error>,
{
    let frame = if !capabilities.images_store {
        response::Frame::Unsupported
    } else {
        match manager.stat(client_identity, &request.name).await {
            Ok(Some(stat)) => response::Frame::Stat(stat),
            Ok(None) => response::Frame::Error(refusal::unknown(&request.name)),
            Err(error) => response::Frame::Error(error.into()),
        }
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
