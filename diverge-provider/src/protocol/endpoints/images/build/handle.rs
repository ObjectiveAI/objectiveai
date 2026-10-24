//! Answering a build, from a scope, the image manager and the volume
//! manager.

use diverge_sdk::shared::Identity;
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
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use crate::protocol::hold::{Allow, Holder};
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Build the image, telling the build as it goes, and end the scope.
///
/// In order:
///
/// 0. [`Unsupported`](response::Frame::Unsupported), then the finish,
///    on a provider whose capabilities keep no image store; nothing
///    is looked at.
/// 1. The context, when the request names one, held: its path
///    checked to be one of names ([`volumes::path`] otherwise), the
///    volume [`got`](VolumeManager::get) ([`volumes::unknown`] for a
///    name the caller has no volume by), and its shared hold
///    [`taken`](crate::protocol::volume::Volume::mount) as a
///    [`Holder::Run`] under what the capabilities allow a run of the
///    volume's mode — [`ContextHeld`](response::Frame::ContextHeld)
///    when it cannot be, whoever has it. An error here is the one
///    [`Error`](response::Frame::Error); either way the scope
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
    client_identity: &Identity,
    manager: &M,
    volume_manager: &V,
    changes: Arc<Changes>,
    capabilities: &Capabilities,
) where
    M: ImageManager,
    M::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    if !capabilities.images_store {
        send(&scope, &response::Frame::Unsupported).await;
        scope.send_response_finish().await;
        return;
    }
    let held = match &request.context {
        None => None,
        Some(context) => match hold(volume_manager, client_identity, context, capabilities).await {
            Ok(volume) => Some(volume),
            Err(frame) => {
                send(&scope, &frame).await;
                scope.send_response_finish().await;
                return;
            }
        },
    };
    let context = request.context.as_ref().map(|context| Context {
        client_identity: client_identity.clone(),
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
        volume.unmount(Holder::Run).await;
    }
    scope.send_response_finish().await;
}

/// The context's volume, found and held as a run's mount is — under
/// what the capabilities allow a run of the volume's mode, nothing
/// where they keep no volumes in it — or the one refusal the
/// endpoint answers with, whichever step refused: an error for a
/// path or a name, `ContextHeld` for a hold not taken, whoever has
/// the volume.
async fn hold<V>(
    volume_manager: &V,
    client_identity: &Identity,
    context: &request::Context,
    capabilities: &Capabilities,
) -> Result<V::Volume, response::Frame>
where
    V: VolumeManager,
    V::Error: Into<Error>,
{
    if !names::ok(&context.volume_relative_path) {
        return Err(response::Frame::Error(volumes::path(&context.volume_relative_path)));
    }
    let volume = match volume_manager.get(client_identity, &context.volume_name).await {
        Ok(Some(volume)) => volume,
        Ok(None) => return Err(response::Frame::Error(volumes::unknown(&context.volume_name))),
        Err(error) => return Err(response::Frame::Error(error.into())),
    };
    let admit = |mode| {
        Some(capabilities.volume(mode).map_or(Allow::default(), |allowed| Allow {
            many: allowed.mount_many,
            beside: allowed.mount_and_fuse_serve,
        }))
    };
    match volume.mount(Holder::Run, admit).await {
        Ok(_) => Ok(volume),
        Err(_) => Err(response::Frame::ContextHeld),
    }
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
