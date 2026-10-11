//! Answering a build, from a scope, the image manager and the volume
//! manager.

use std::sync::Arc;

use futures_util::StreamExt as _;

use diverge_sdk::provider::endpoints::images::build::client::request;
use diverge_sdk::provider::endpoints::images::build::server::response;
use diverge_sdk::provider::endpoints::images::refusal;
use diverge_sdk::provider::endpoints::volumes::{names, refusal as volumes};
use crate::protocol::changes::Changes;
use crate::protocol::context::Context;
use crate::protocol::image_manager::ImageManager;
use crate::protocol::progress::Progress;
use crate::protocol::volume::Volume as _;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Build the image, telling the build as it goes, and end the scope.
///
/// In order:
///
/// 1. The context, when the request names one, held: its path
///    checked to be one of names ([`volumes::path`] otherwise), the
///    volume [`got`](VolumeManager::get) ([`volumes::unknown`] for a
///    name the caller has no volume by), and its shared hold
///    [`taken`](crate::protocol::volume::Volume::mount)
///    ([`volumes::held`] when it cannot be — the volume is held
///    exclusively, or is persistent and held at all). A refusal here
///    is the one [`Error`](response::Frame::Error), and the scope
///    finishes with nothing built.
/// 2. The manager asked to build, with the context as a
///    [`Context`] — the request's two fields and the caller's
///    identity — and its stream read to the end: every
///    [`Output`](Progress::Output) sent as it comes; the first of
///    [`Built`](Progress::Built), told to every listing open by name,
///    [`InsufficientCapacity`](Progress::InsufficientCapacity) and the
///    stream's error, flattened, sent and the stream left. A stream
///    that ends with none of them is [`refusal::unfinished`].
/// 3. The hold given back, and the finish.
///
/// A response that will not encode is the one failure with nowhere to
/// go, and is not sent. The request arrives decoded, as every
/// handler's does.
pub async fn handle<M, V>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
    volume_manager: &V,
    changes: Arc<Changes>,
) where
    M: ImageManager,
    M::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    let held = match &request.context {
        None => None,
        Some(context) => match hold(volume_manager, client_identity, context).await {
            Ok(volume) => Some(volume),
            Err(error) => {
                send(&scope, &response::Frame::Error(error)).await;
                scope.send_response_finish().await;
                return;
            }
        },
    };
    let context = request.context.as_ref().map(|context| Context {
        client_identity: client_identity.to_string(),
        volume_name: context.volume_name.clone(),
        volume_relative_path: context.volume_relative_path.clone(),
    });
    let build = manager
        .build(client_identity, &request.name, &request.containerfile, context.as_ref())
        .await;
    let mut build = std::pin::pin!(build);
    let mut ended = false;
    while let Some(item) = build.next().await {
        let frame = match item {
            Ok(Progress::Output(line)) => response::Frame::Output(line),
            Ok(Progress::Built(built)) => {
                changes.changed(client_identity, &request.name);
                ended = true;
                response::Frame::Built(built)
            }
            Ok(Progress::InsufficientCapacity) => {
                ended = true;
                response::Frame::InsufficientCapacity
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
    if !ended {
        send(&scope, &response::Frame::Error(refusal::unfinished())).await;
    }
    if let Some(volume) = held {
        volume.unmount().await;
    }
    scope.send_response_finish().await;
}

/// The context's volume, found and held as a mount is; or the one
/// error the endpoint answers with, whichever step refused.
async fn hold<V>(volume_manager: &V, client_identity: &str, context: &request::Context) -> Result<V::Volume, Error>
where
    V: VolumeManager,
    V::Error: Into<Error>,
{
    if !names::ok(&context.volume_relative_path) {
        return Err(volumes::path(&context.volume_relative_path));
    }
    let volume = volume_manager
        .get(client_identity, &context.volume_name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| volumes::unknown(&context.volume_name))?;
    if !volume.mount().await {
        return Err(volumes::held(&context.volume_name));
    }
    Ok(volume)
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
