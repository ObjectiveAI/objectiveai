//! The conduit: a request put on another daemon's connection, and
//! every frame carried both ways with the numbers remapped.

use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::wire::client::handle::{Handle, SendError};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::frame::server::ServerFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinSet;

/// Put `bytes` on `handle` as a request of its own, and carry the
/// exchange between the client's scope and the remote's until the
/// remote finishes: every response the remote sends is sent on the
/// client's scope; every channel the client opens is opened on the
/// remote's scope, and its responses carried back under the client's
/// number; every channel the remote opens is opened on the client's
/// scope, and the client's responses carried over under the remote's
/// number. Each side minted its own numbers, and each pair lives in
/// the task that carries that channel. The client's finish is sent
/// when the remote's arrives, and every task still carrying ends
/// then. `Err` only when the connection would not take the request:
/// nothing was sent to the client.
pub async fn run(scope: Arc<ScopeHandle>, handle: &Handle, bytes: &[u8]) -> Result<(), SendError> {
    let mut remote = handle.send_request(bytes).await?;
    let tasks = Arc::new(Mutex::new(JoinSet::new()));
    {
        let mut set = tasks.lock().await;
        set.spawn(client_to_remote(Arc::clone(&scope), handle.clone(), remote.scope, Arc::clone(&tasks)));
        set.spawn(remote_to_client(Arc::clone(&scope), handle.clone(), remote.scope, remote.request_receiver, Arc::clone(&tasks)));
    }
    while let Some(frame) = remote.response_receiver.recv().await {
        match ServerFrame::decode(&frame) {
            Ok(ServerFrame::Response { payload, .. }) => scope.send_response(payload).await,
            Ok(ServerFrame::ResponseFinish { .. }) => break,
            _ => {}
        }
    }
    tasks.lock().await.abort_all();
    scope.send_response_finish().await;
    Ok(())
}

/// Every channel the client opens on its scope, opened on the remote's
/// scope: the request carried over, and a task carrying the remote's
/// responses back under the client's number. Ends when the client's
/// scope stops yielding requests — its finish sent, or the client
/// gone.
async fn client_to_remote(scope: Arc<ScopeHandle>, handle: Handle, remote: u32, tasks: Arc<Mutex<JoinSet<()>>>) {
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let Ok(opened) = handle.send_channel_request(remote, payload).await else {
            scope.send_channel_response_finish(channel).await;
            continue;
        };
        tasks.lock().await.spawn(channel_responses_back(Arc::clone(&scope), channel, opened.response_receiver));
    }
}

/// The remote's responses on one channel the client opened, sent to
/// the client under the client's number, to the finish.
async fn channel_responses_back(scope: Arc<ScopeHandle>, channel: u32, mut responses: UnboundedReceiver<Bytes>) {
    while let Some(frame) = responses.recv().await {
        match ServerFrame::decode(&frame) {
            Ok(ServerFrame::ChannelResponse { payload, .. }) => scope.send_channel_response(channel, payload).await,
            Ok(ServerFrame::ChannelResponseFinish { .. }) => {
                scope.send_channel_response_finish(channel).await;
                return;
            }
            _ => {}
        }
    }
}

/// Every channel the remote opens inside its scope, opened on the
/// client's scope: the request carried over, and a task carrying the
/// client's responses over under the remote's number. Ends when the
/// remote's scope stops yielding requests.
async fn remote_to_client(scope: Arc<ScopeHandle>, handle: Handle, remote: u32, mut requests: UnboundedReceiver<Bytes>, tasks: Arc<Mutex<JoinSet<()>>>) {
    while let Some(frame) = requests.recv().await {
        let Ok(ServerFrame::ChannelRequest { channel, payload, .. }) = ServerFrame::decode(&frame) else {
            continue;
        };
        let opened = scope.send_channel_request(payload).await;
        tasks.lock().await.spawn(channel_responses_over(handle.clone(), remote, channel, opened));
    }
}

/// The client's responses on one channel the remote opened, sent to
/// the remote under the remote's number, to the finish. The channel
/// is held here for as long as it is carried, since its dropping is
/// its closing on the client's side.
async fn channel_responses_over(handle: Handle, remote: u32, channel: u32, mut opened: diverge_sdk::wire::server::channel::Channel) {
    while let Some(bytes) = opened.response_receiver.recv().await {
        match ClientFrame::decode(&bytes) {
            Ok(ClientFrame::ChannelResponse { payload, .. }) => {
                if handle.send_channel_response(remote, channel, payload).await.is_err() {
                    return;
                }
            }
            Ok(ClientFrame::ChannelResponseFinish { .. }) => {
                let _ = handle.send_channel_response_finish(remote, channel).await;
                return;
            }
            _ => {}
        }
    }
}
