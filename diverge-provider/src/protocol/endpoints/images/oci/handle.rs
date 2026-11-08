//! Answering a pull, from a scope and the store.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::task::JoinSet;

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::provider::endpoints::images::oci::client::{channel_request, request};
use diverge_sdk::shared::containers::oci;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::oci_store::{OciStore, ServedImage};
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::provider::endpoints::images::oci::server::response;

/// Serve the one image the request names, every ask the caller opens
/// on it, until the caller cancels.
///
/// The rules:
///
/// 1. **Nowhere to look.** A provider whose capabilities keep no
///    Store and list no registry sends
///    [`Unsupported`](response::Frame::Unsupported) on the main
///    stream and finishes, serving no channel.
/// 2. **Not found.** The store opens the image for the caller once; an
///    image it does not serve to the caller is
///    [`NotFound`](response::Frame::NotFound), sent first and alone,
///    and the finish.
/// 3. **Served.** Otherwise nothing is sent on the main stream, and
///    each channel the caller opens is answered on its own task: a
///    manifest with at most one frame then the finish; a blob with its
///    pieces, each split at [`CHUNK_SIZE`], then the finish — the
///    empty finish being a digest the image does not reach, or one the
///    store could not supply. A channel request that will not decode
///    is finished with nothing. An answer done is reaped as it ends,
///    so the set holds only the answers in flight however long the
///    scope is held.
/// 4. **The end.** The caller's `Cancel`, the caller gone, or the
///    caller reading no more of the scope ends the loop; every answer
///    still in flight is finished, and then the scope — save that a
///    caller reading no more of the scope is not waited on, and what
///    is still in flight for it is ended. The image is
///    [`close`](ServedImage::close)d after the finish, once nothing
///    asks of it.
///
/// # `client_identity` is an argument
///
/// Because what the store serves one caller is not what it serves
/// another, and the connection is what knows whose scope this is.
pub async fn handle<O>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
    store: &O,
    capabilities: &Capabilities,
) where
    O: OciStore,
{
    if !capabilities.images_store && capabilities.images_registries.is_empty() {
        refuse(scope, &response::Frame::Unsupported).await;
        return;
    }
    let Some(served) = store.open(client_identity, &request.0.image).await else {
        refuse(scope, &response::Frame::NotFound).await;
        return;
    };
    let scope = Arc::new(scope);
    let served = Arc::new(served);
    let mut tasks = JoinSet::new();
    loop {
        let bytes = tokio::select! {
            received = scope.recv_channel_request() => match received {
                Some(bytes) => bytes,
                None => break,
            },
            () = scope.responses_gone() => break,
            Some(_) = tasks.join_next(), if !tasks.is_empty() => continue,
        };
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::OciManifest(request)) => {
                tasks.spawn(manifest(Arc::clone(&scope), Arc::clone(&served), channel, request.digest));
            }
            Ok(channel_request::Frame::OciBlob(request)) => {
                tasks.spawn(blob(Arc::clone(&scope), Arc::clone(&served), channel, request.digest));
            }
            Ok(channel_request::Frame::Cancel) => break,
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }
    let gone = tokio::select! {
        () = drain(&mut tasks) => false,
        () = scope.responses_gone() => true,
    };
    if gone {
        tasks.abort_all();
        drain(&mut tasks).await;
    }
    scope.send_response_finish().await;
    served.close().await;
}

/// One frame on the main stream, then the finish: the scope over
/// before any channel is served.
async fn refuse(scope: ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// Every task of `tasks` waited for, to the last.
async fn drain(tasks: &mut JoinSet<()>) {
    while tasks.join_next().await.is_some() {}
}

/// At most one frame — the manifest's media type and bytes — then
/// the finish.
async fn manifest<I: ServedImage>(scope: Arc<ScopeHandle>, served: Arc<I>, channel: u32, digest: String) {
    if let Some(manifest) = served.manifest(&digest).await {
        let frame = oci::manifest::response::Frame {
            media_type: &manifest.media_type,
            body: &manifest.body,
        };
        respond(&scope, channel, &frame).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// The blob's pieces, each split at the chunk size, then the finish.
async fn blob<I: ServedImage>(scope: Arc<ScopeHandle>, served: Arc<I>, channel: u32, digest: String) {
    if let Some(pieces) = served.blob(&digest).await {
        let mut pieces = std::pin::pin!(pieces);
        while let Some(piece) = pieces.next().await {
            for body in piece.chunks(CHUNK_SIZE) {
                respond(&scope, channel, &oci::blob::response::Frame { body }).await;
            }
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// One channel response, when it encoded; one that will not is not
/// sent, and the finish that follows says so.
async fn respond<T: Encode>(scope: &ScopeHandle, channel: u32, frame: &T) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_channel_response(channel, &buffer).await;
    }
}
