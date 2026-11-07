//! Holding a daemon's accept scope, from a scope and the acceptors.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::accept::server::response::{self, Accepting};
use crate::protocol::endpoints::containers::run::encoded::encoded;
use diverge_sdk::provider::endpoints::daemons::accept::client::channel_request;
use crate::protocol::acceptors::Acceptors;
use crate::protocol::release::Release;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::singles::Single;

/// Hold the identity's accept slot for the scope's life, and pair the
/// halves of every connection relayed on it.
///
/// In order:
///
/// 1. The slot, taken under the connection's identity — or not, which
///    is [`AlreadyOpen`](response::Frame::AlreadyOpen), then the
///    finish. The connection's own claim on its one accept was made
///    before this was called, and refused there as the same answer.
/// 2. Exactly one response, [`Accepting`] with the identity.
/// 3. For the scope's life, every channel the daemon opens: its half
///    of a connection, quoting an id — handed to the connect handler
///    waiting on that id, which relays the connector's frames onto
///    it from then on; an id nobody waits for is finished with
///    nothing — or the cancel, which ends the scope. A channel request
///    that does not decode is finished with nothing. The daemon
///    reading no more of the scope ends it as the cancel does.
/// 4. The slot given back, which ends every connection relayed on
///    the scope, then the connection's claim, and the finish. The
///    giving back is a [`Release`], so a scope that panics or is
///    cancelled gives the slot back at the drop; the claim is held
///    with the scope, and goes before its finish either way — see
///    [`Single`].
///
/// The provider's half of each connection is opened by the connect
/// handler, on this scope, and read there; nothing here reads it.
pub async fn handle(scope: Single<ScopeHandle>, client_identity: &Identity, acceptors: Arc<Acceptors>) {
    let scope = scope.map(Arc::new);
    let Some(accepting) = acceptors.take(client_identity, Arc::clone(scope.scope())).await else {
        send(&scope, response::Frame::AlreadyOpen).await;
        scope.send_response_finish().await;
        return;
    };
    let slot = Release::new({
        let acceptors = Arc::clone(&acceptors);
        let identity = client_identity.clone();
        async move { acceptors.release(&identity).await }
    });
    send(
        &scope,
        response::Frame::Accepting(Accepting {
            identity: client_identity.clone(),
        }),
    )
    .await;
    loop {
        let bytes = tokio::select! {
            received = scope.recv_channel_request() => match received {
                Some(bytes) => bytes,
                None => break,
            },
            () = scope.responses_gone() => break,
        };
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Connection(connection)) => {
                if !accepting.deliver(connection.connection_id, channel).await {
                    scope.send_channel_response_finish(channel).await;
                }
            }
            Ok(channel_request::Frame::Cancel) => break,
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }
    slot.now().await;
    scope.send_response_finish().await;
}

/// An accept on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// One response on the scope.
async fn send(scope: &ScopeHandle, frame: response::Frame) {
    if let Some(payload) = encoded(&frame) {
        scope.send_response(&payload).await;
    }
}
