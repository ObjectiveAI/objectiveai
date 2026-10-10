//! Holding a daemon's accept scope, from a scope and the acceptors.

use std::sync::Arc;

use super::super::response::{self, Accepting};
use crate::provider::endpoints::containers::server::encoded::encoded;
use crate::provider::endpoints::daemons::accept::client::channel_request;
use crate::provider::server::acceptors::Acceptors;
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;
use crate::wire::frame::client::ClientFrame;
use crate::wire::server::scope_handle::ScopeHandle;

/// Hold the identity's accept slot for the scope's life, and pair the
/// halves of every connection relayed on it.
///
/// In order:
///
/// 1. The slot, taken under the connection's identity — or not, which
///    is the scope's one `Error`, `{"kind":"held"}`, then the finish.
/// 2. Exactly one response, [`Accepting`] with the identity.
/// 3. For the scope's life, every channel the daemon opens: its half
///    of a connection, quoting an id — handed to the connect handler
///    waiting on that id, which relays the connector's frames onto
///    it from then on; an id nobody waits for is finished with
///    nothing — or the stop, which ends the scope. A channel request
///    that does not decode is finished with nothing.
/// 4. The slot given back, which ends every connection relayed on
///    the scope, and the finish.
///
/// The provider's half of each connection is opened by the connect
/// handler, on this scope, and read there; nothing here reads it.
pub async fn handle(scope: ScopeHandle, client_identity: &str, acceptors: Arc<Acceptors>) {
    let scope = Arc::new(scope);
    let Some(accepting) = acceptors.take(client_identity, Arc::clone(&scope)).await else {
        send(&scope, response::Frame::Error(held())).await;
        scope.send_response_finish().await;
        return;
    };
    send(
        &scope,
        response::Frame::Accepting(Accepting {
            identity: client_identity.to_string(),
        }),
    )
    .await;
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Connection(connection)) => {
                if !accepting.deliver(connection.connection_id, channel).await {
                    scope.send_channel_response_finish(channel).await;
                }
            }
            Ok(channel_request::Frame::Stop) => break,
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }
    acceptors.release(client_identity).await;
    scope.send_response_finish().await;
}

/// One response on the scope.
async fn send(scope: &ScopeHandle, frame: response::Frame) {
    if let Some(payload) = encoded(&frame) {
        scope.send_response(&payload).await;
    }
}

/// The identity accepts here already.
fn held() -> Error {
    Error(serde_json::json!({ "kind": "held" }))
}
