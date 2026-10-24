//! Answering a pull, from a scope and the store.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::task::JoinSet;

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::provider::endpoints::images::oci::client::channel_request;
use diverge_sdk::shared::containers::oci;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::oci_store::OciStore;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::provider::endpoints::images::oci::server::response;

/// Answer every ask the caller opens, until it disconnects.
///
/// Nothing is sent on the main stream — but on a provider whose
/// capabilities keep no image store, which sends
/// [`Unsupported`](response::Frame::Unsupported) there and finishes,
/// serving no channel. Otherwise the scope is held, and each
/// channel the caller opens is answered on its own task — a has with
/// one frame, the name or not held, then the finish; a manifest with
/// at most one frame then the finish; a blob with its pieces, each
/// split at [`CHUNK_SIZE`], then the finish — the empty finish being
/// a digest the store does not hold for the caller. A channel request
/// that will not decode is finished with nothing. The caller's
/// disconnect, or the caller gone, ends the loop; every answer still
/// in flight is finished, and then the scope.
///
/// # `client_identity` is an argument
///
/// Because what the store holds for one caller is not what it holds
/// for another, and the connection is what knows whose scope this
/// is.
pub async fn handle<O>(scope: ScopeHandle, client_identity: &Identity, store: Arc<O>, capabilities: &Capabilities)
where
    O: OciStore + 'static,
{
    if !capabilities.images_store {
        let mut buffer = Vec::new();
        response::Frame::Unsupported
            .encode(&mut Writer::new(&mut buffer))
            .unwrap_or_else(|error| match error {});
        scope.send_response(&buffer).await;
        scope.send_response_finish().await;
        return;
    }
    let scope = Arc::new(scope);
    let identity: Arc<Identity> = Arc::new(client_identity.clone());
    let mut tasks = JoinSet::new();
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::OciHas(request)) => {
                tasks.spawn(has(Arc::clone(&scope), Arc::clone(&store), Arc::clone(&identity), channel, request.digest));
            }
            Ok(channel_request::Frame::OciManifest(request)) => {
                tasks.spawn(manifest(Arc::clone(&scope), Arc::clone(&store), Arc::clone(&identity), channel, request.digest));
            }
            Ok(channel_request::Frame::OciBlob(request)) => {
                tasks.spawn(blob(Arc::clone(&scope), Arc::clone(&store), Arc::clone(&identity), channel, request.digest));
            }
            Ok(channel_request::Frame::Disconnect) => break,
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }
    while tasks.join_next().await.is_some() {}
    scope.send_response_finish().await;
}

/// One frame — held under a name, or not — then the finish.
async fn has<O: OciStore>(scope: Arc<ScopeHandle>, store: Arc<O>, identity: Arc<Identity>, channel: u32, digest: String) {
    let name = store.holds(&identity, &digest).await;
    respond(&scope, channel, &oci::has::response::Frame { name }).await;
    scope.send_channel_response_finish(channel).await;
}

/// At most one frame — the manifest's media type and bytes — then
/// the finish.
async fn manifest<O: OciStore>(scope: Arc<ScopeHandle>, store: Arc<O>, identity: Arc<Identity>, channel: u32, digest: String) {
    if let Some(manifest) = store.manifest(&identity, &digest).await {
        let frame = oci::manifest::response::Frame {
            media_type: &manifest.media_type,
            body: &manifest.body,
        };
        respond(&scope, channel, &frame).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// The blob's pieces, each split at the chunk size, then the finish.
async fn blob<O: OciStore>(scope: Arc<ScopeHandle>, store: Arc<O>, identity: Arc<Identity>, channel: u32, digest: String) {
    if let Some(pieces) = store.blob(&identity, &digest).await {
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
