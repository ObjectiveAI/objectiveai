//! Answering a hash, from a scope and a manager.

use diverge_sdk::shared::Identity;

use crate::protocol::image_manager::ImageManager;
use diverge_sdk::provider::endpoints::images::hash::client::request;
use diverge_sdk::provider::endpoints::images::hash::server::response;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Compute the proof of possession over the image the request names,
/// as this provider serves it to the caller, and end the scope.
///
/// The rules:
///
/// 1. A provider whose capabilities keep no Store and list no
///    registry answers [`Unsupported`](response::Frame::Unsupported)
///    and asks nothing.
/// 2. The manager resolves the image as an `images::oci` scope would —
///    the caller's own Store, else a registry the provider uses that
///    one of its references names — and computes `SHA-256(nonce ‖ D)`
///    over its own bytes of it, those of a registry brought in first —
///    the computation of
///    [`prove`](diverge_sdk::shared::containers::oci::prove), the same
///    the provider's own nonce check makes — answered as
///    [`Hash`](response::Frame::Hash).
/// 3. An image served to the caller from neither is
///    [`NotFound`](response::Frame::NotFound), and so is a `platform`
///    that breaks the rule: absent for an index, present for an image
///    manifest, or not one of the manifests the index lists. No proof
///    is ever computed over an index alone.
/// 4. A manager that will not answer is its error, flattened.
///
/// So a daemon whose image this provider serves answers another
/// provider's nonce check by relaying the request here and the 32
/// bytes back: no image byte moves between them.
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
    let frame = if !capabilities.images_store && capabilities.images_registries.is_empty() {
        response::Frame::Unsupported
    } else {
        match manager.hash(client_identity, &request.0).await {
            Ok(Some(proof)) => response::Frame::Hash { proof },
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
