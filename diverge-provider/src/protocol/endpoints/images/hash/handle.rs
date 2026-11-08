//! Answering a hash, from a scope and a manager.

use diverge_sdk::shared::Identity;

use crate::protocol::image_manager::ImageManager;
use diverge_sdk::provider::endpoints::images::hash::client::request;
use diverge_sdk::provider::endpoints::images::hash::server::response;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Compute the proof of possession over the caller's own image and end
/// the scope.
///
/// The rules:
///
/// 1. A provider whose capabilities keep no Store answers
///    [`Unsupported`](response::Frame::Unsupported) and asks nothing.
/// 2. The manager computes `SHA-256(nonce ‖ D)` over the Store's own
///    bytes of the caller's image under the digest — the computation
///    of [`prove`](diverge_sdk::shared::containers::oci::prove), the
///    same the provider's own nonce check makes — answered as
///    [`Hash`](response::Frame::Hash).
/// 3. A digest the caller owns no image by is
///    [`NotFound`](response::Frame::NotFound), and so is a `platform`
///    that breaks the rule: absent for an index, present for an image
///    manifest, or not one of the manifests the index lists and the
///    Store holds. No proof is ever computed over an index alone.
/// 4. A manager that will not answer is its error, flattened.
///
/// So a daemon whose image is in this provider's Store answers another
/// provider's nonce check by relaying the nonce here and the 32 bytes
/// back: no image byte moves.
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
        match manager
            .hash(client_identity, &request.digest, request.platform.as_deref(), &request.nonce)
            .await
        {
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
