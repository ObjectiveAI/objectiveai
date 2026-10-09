//! Answering a container serve, from a scope and the directory.

use std::sync::Arc;

use bytes::Bytes;
use tokio::task::JoinSet;

use super::super::response;
use crate::container_proxy::outside::endpoints::filesystem::serve::client::execute as proxy;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::serve::client::{channel_request, request};
use crate::wire::frame::client::ClientFrame;
use crate::wire::frame::server::ServerFrame;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::provider::server::directory::Directory;
use crate::shared::error::Error;

/// Find the container, open the proxy's serve of the subtree, relay
/// every ask and every tree, and end the scope at the stop.
///
/// In order:
///
/// 1. The container, found in the [`Directory`] by its id and
///    checked to be the caller's own — the runner, not a connector —
///    or not, which is the scope's one `Error`, `{"kind":"missing"}`
///    either way, then the finish.
/// 2. A [`filesystem::serve`](crate::container_proxy::outside::endpoints::filesystem::serve)
///    scope opened on the container's proxy, carrying the path. The
///    proxy's refusal — a path that is not a directory, or one it
///    leaves out — is relayed as the scope's one `Error`, in the
///    proxy's own words, then the finish.
/// 3. `Serving`, once the proxy said so.
/// 4. Every channel the caller opens, relayed: an ask is one channel
///    on the proxy's serve scope carrying the caller's payload as it
///    came, and the proxy's one answer and finish come back on the
///    caller's channel as they came; a filetree is the proxy's
///    filetree channel, frame for frame, until the proxy finishes it.
///    Each on a task of its own, so asks in flight at once are
///    answered in whatever order the proxy answers them; a channel
///    request that will not decode is finished with nothing.
/// 5. The end: the caller's stop, the caller's connection ending, or
///    the container ending, heard through the directory. The proxy's
///    serve is stopped — it answers what is in flight and finishes,
///    which ends every filetree relay — the tasks are awaited, and
///    the scope is finished. Nothing is released: the container is
///    its runner's.
///
/// # The conduit
///
/// Nothing here reads an ask or an answer. The channel request is
/// decoded once to tell an ask from the stop and the tree, and the
/// bytes relayed are the bytes that arrived.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(scope: ScopeHandle, request: request::Frame, client_identity: &str, directory: Arc<Directory>) {
    let Some(attached) = directory.lookup(&request.id).await else {
        return refuse(scope, missing()).await;
    };
    if !directory.runs(&request.id, client_identity).await {
        return refuse(scope, missing()).await;
    }
    let served = match proxy::execute(&attached.proxy, request.path).await {
        Ok(served) => Arc::new(served),
        Err(proxy::ExecuteError::Refused(error)) => return refuse(scope, error).await,
        Err(error) => return refuse(scope, unreachable(&error)).await,
    };
    send(&scope, &response::Frame::Serving).await;

    let scope = Arc::new(scope);
    let mut ended = attached.ended;
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            received = scope.recv_channel_request() => {
                let Some(bytes) = received else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Ask(_)) => {
                        let ask = bytes.slice_ref(payload);
                        tasks.spawn(relay_ask(Arc::clone(&scope), Arc::clone(&served), channel, ask));
                    }
                    Ok(channel_request::Frame::Filetree) => {
                        tasks.spawn(relay_tree(Arc::clone(&scope), Arc::clone(&served), channel));
                    }
                    Ok(channel_request::Frame::Stop) => break,
                    Err(_) => {
                        scope.send_channel_response_finish(channel).await;
                    }
                }
            }
            // The run over, or the directory gone with the provider:
            // the serve ends either way.
            changed = ended.changed() => {
                if changed.is_err() || *ended.borrow_and_update() {
                    break;
                }
            }
        }
    }
    served.stop().await;
    while tasks.join_next().await.is_some() {}
    scope.send_response_finish().await;
}

/// The one response a refused serve gets, then the finish.
async fn refuse(scope: ScopeHandle, error: Error) {
    send(&scope, &response::Frame::Error(error)).await;
    scope.send_response_finish().await;
}

/// One response on channel `0`, encoded and sent; one that will not
/// encode is not sent.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}

/// One ask relayed: the caller's payload as the proxy's channel
/// request, the proxy's one answer as the caller's channel response,
/// then the finish. A proxy that could not serve the ask, or that is
/// gone, is the finish with nothing.
async fn relay_ask(scope: Arc<ScopeHandle>, served: Arc<proxy::ExecuteHandle>, channel: u32, ask: Bytes) {
    if let Ok(answer) = served.relay(&ask).await {
        scope.send_channel_response(channel, &answer).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// One filetree channel relayed: the proxy's own filetree channel,
/// every frame as it came, until the proxy finishes it — which it
/// does when its scope ends — or is gone; then the finish.
async fn relay_tree(scope: Arc<ScopeHandle>, served: Arc<proxy::ExecuteHandle>, channel: u32) {
    if let Ok(mut tree) = served.filetree().await {
        while let Some(bytes) = tree.response_receiver.recv().await {
            match ServerFrame::decode(&bytes) {
                Ok(ServerFrame::ChannelResponse { payload, .. }) => {
                    scope.send_channel_response(channel, payload).await;
                }
                Ok(ServerFrame::ChannelResponseFinish { .. }) => break,
                _ => {}
            }
        }
    }
    scope.send_channel_response_finish(channel).await;
}

/// No container of the caller's runs under that id: none at all, or
/// one the caller is not running.
fn missing() -> Error {
    Error(serde_json::json!({ "kind": "missing" }))
}

/// The proxy did not answer the serve: the connection to it is gone,
/// or it answered something this end cannot read.
fn unreachable(error: &proxy::ExecuteError) -> Error {
    Error(serde_json::json!({ "kind": "proxy", "error": error.to_string() }))
}
