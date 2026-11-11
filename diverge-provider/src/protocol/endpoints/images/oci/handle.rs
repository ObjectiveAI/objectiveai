//! Answering whether an image is held, and a pull of it, from a scope
//! and the store.

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
use diverge_sdk::shared::error::Error;

/// Say whether the one image the request names is held, and serve it,
/// every ask the caller opens on it, until the caller cancels.
///
/// The scope is the has: its first frame is the answer, and a held
/// image is served on the same scope, so a caller that asked never
/// opens a second to pull.
///
/// The rules:
///
/// 1. **Nowhere to look.** A provider whose capabilities keep no
///    Store and list no registry sends
///    [`Unsupported`](response::Frame::Unsupported) on the main
///    stream and finishes, serving no channel; the store is never
///    asked.
/// 2. **Not found.** The store opens the image for the caller once; an
///    image it does not serve to the caller is
///    [`NotFound`](response::Frame::NotFound), sent first and alone,
///    and the finish — an answer: the provider looked and will not
///    supply the image to this caller.
/// 3. **No answer.** An open that fails is
///    [`Error`](response::Frame::Error), flattened, sent first and
///    alone, and the finish: the absence of an answer, which a caller
///    that treated as a no would conclude an image is not held when it
///    was never told. Nothing here turns a failure into a no.
/// 4. **Held.** Otherwise [`Held`](response::Frame::Held) is sent
///    first, before any channel is answered, and nothing more on the
///    main stream until the finish. Each channel the caller opens is
///    answered on its own task: a manifest with at most one frame then
///    the finish; a blob with its pieces, each split at
///    [`CHUNK_SIZE`], then the finish; a proof with one frame of its
///    32 bytes then the finish — the empty finish being a digest the
///    image does not reach, one the store could not supply, or a proof
///    the image could not give. A channel request that will not decode
///    is finished with nothing. An answer done is reaped as it ends,
///    so the set holds only the answers in flight however long the
///    scope is held.
/// 5. **The end.** The caller's `Cancel`, the caller gone, or the
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
    O::Error: Into<Error>,
{
    if !capabilities.images_store && capabilities.images_registries.is_empty() {
        refuse(scope, &response::Frame::Unsupported).await;
        return;
    }
    let served = match store.open(client_identity, &request.0.image).await {
        Ok(Some(served)) => served,
        Ok(None) => {
            refuse(scope, &response::Frame::NotFound).await;
            return;
        }
        Err(error) => {
            refuse(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    send(&scope, &response::Frame::Held).await;
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
            Ok(channel_request::Frame::OciProve(request)) => {
                tasks.spawn(prove(Arc::clone(&scope), Arc::clone(&served), channel, request));
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
    send(&scope, frame).await;
    scope.send_response_finish().await;
}

/// One frame on the main stream, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
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

/// At most one frame — the proof — then the finish. The request's
/// image is the scope's, so only its platform and nonce are read.
async fn prove<I: ServedImage>(scope: Arc<ScopeHandle>, served: Arc<I>, channel: u32, request: oci::prove::request::Request) {
    if let Some(proof) = served.prove(request.platform.as_deref(), &request.nonce).await {
        respond(&scope, channel, &oci::prove::response::Frame { proof }).await;
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
