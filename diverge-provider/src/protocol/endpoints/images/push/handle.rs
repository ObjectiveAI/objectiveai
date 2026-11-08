//! Answering a push, from a scope and the image manager.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use futures_util::StreamExt as _;
use serde_json::json;

use crate::protocol::changes::Changes;
use crate::protocol::image_manager::ImageManager;
use crate::protocol::image_source::ImageSource;
use crate::protocol::progress::Pushing;
use diverge_sdk::provider::endpoints::images::push::client::request;
use diverge_sdk::provider::endpoints::images::push::server::{channel_request, response};
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Take the image into the caller's Store, telling every blob taken
/// from the caller as it lands, and end the scope.
///
/// In order:
///
/// 0. [`Unsupported`](response::Frame::Unsupported), then the finish,
///    on a provider whose capabilities keep no Store; nothing is
///    looked at.
/// 1. The manager asked to push the request's image with its
///    description, handed the caller's store over this scope — the
///    push's three asks, `OciManifest`, `OciBlob` and `OciProve`, each
///    one channel this end opens — and its stream read to the end:
///    every [`Blob`](Pushing::Blob) sent as it comes; the first end —
///    [`Pushed`](Pushing::Pushed), told to every listing open by
///    digest, or `Exists`, `InsufficientCapacity`, `Unavailable`,
///    `ProofFailed`, `Malformed`, or the stream's error, flattened —
///    sent and the stream left. A stream that ends with none of them
///    is the error that the push ended without a result. A caller
///    that reads no more of the scope leaves the stream too, with
///    nothing sent.
/// 2. The finish.
///
/// The push is not one of the endpoints held one at a time per
/// connection: two pushes on one connection run beside each other,
/// and the Store decides which of two pushes of one digest owns it.
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
    if !capabilities.images_store {
        send(&scope, &response::Frame::Unsupported).await;
        scope.send_response_finish().await;
        return;
    }
    let scope = Arc::new(scope);
    let source = ImageSource::new(Arc::clone(&scope), manifest_ask, blob_ask, prove_ask);
    let push = manager
        .push(client_identity, &request.image, request.description.as_ref(), source)
        .await;
    let mut push = Box::pin(push);
    let mut ended = false;
    loop {
        let item = tokio::select! {
            item = push.next() => item,
            () = scope.responses_gone() => {
                ended = true;
                break;
            }
        };
        let Some(item) = item else {
            break;
        };
        let frame = match item {
            Ok(Pushing::Blob(blob)) => response::Frame::Blob(blob),
            Ok(Pushing::Pushed(pushed)) => {
                changes.changed(client_identity, &pushed.digest);
                ended = true;
                response::Frame::Pushed(pushed)
            }
            Ok(Pushing::Exists(exists)) => {
                ended = true;
                response::Frame::Exists(exists)
            }
            Ok(Pushing::InsufficientCapacity) => {
                ended = true;
                response::Frame::InsufficientCapacity
            }
            Ok(Pushing::Unavailable) => {
                ended = true;
                response::Frame::Unavailable
            }
            Ok(Pushing::ProofFailed) => {
                ended = true;
                response::Frame::ProofFailed
            }
            Ok(Pushing::Malformed) => {
                ended = true;
                response::Frame::Malformed
            }
            Err(error) => {
                ended = true;
                response::Frame::Error(error.into())
            }
        };
        send(&scope, &frame).await;
        if ended {
            break;
        }
    }
    drop(push);
    if !ended {
        send(&scope, &response::Frame::Error(unfinished())).await;
    }
    scope.send_response_finish().await;
}

/// The error a push that ended without a result answers.
fn unfinished() -> Error {
    Error(json!({
        "kind": "push",
        "error": "the push ended without a result",
    }))
}

/// The ask for a manifest the provider does not hold, as the push's
/// channel request.
fn manifest_ask(digest: &str) -> Vec<u8> {
    encoded(&channel_request::Frame::OciManifest(oci::manifest::request::Request {
        digest: digest.to_string(),
    }))
}

/// The ask for a blob the provider does not hold, as the push's
/// channel request.
fn blob_ask(digest: &str) -> Vec<u8> {
    encoded(&channel_request::Frame::OciBlob(oci::blob::request::Request {
        digest: digest.to_string(),
    }))
}

/// The nonce check's ask, as the push's channel request.
fn prove_ask(request: &oci::prove::request::Request) -> Vec<u8> {
    encoded(&channel_request::Frame::OciProve(request.clone()))
}

/// One frame's bytes; one that will not encode is empty, which the
/// caller cannot read and answers as nothing.
fn encoded<T: Encode>(frame: &T) -> Vec<u8> {
    let mut buffer = Vec::new();
    match frame.encode(&mut Writer::new(&mut buffer)) {
        Ok(()) => buffer,
        Err(_) => Vec::new(),
    }
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
