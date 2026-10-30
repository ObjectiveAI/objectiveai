//! What the container's server says on its own account on one
//! caller's session, for as long as the channel lives.

use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::mcp::notifications::{request, response};
use diverge_sdk::shared::mcp::session::Sessioned;
use tokio::sync::broadcast;

use crate::proxy::Proxy;
use crate::encode::encoded;
use crate::reply::reply;

/// Subscribe to the caller's session, then relay: one frame per
/// notification sent on it, for the connection's life. The session's
/// client is dialled first so a container
/// whose server cannot be reached answers `Error` at once rather than
/// a silence that could mean anything; a subscriber that fell behind
/// loses the oldest and goes on, which the stream does not report — a
/// notification is a nudge, not a record. Only the responder finishes
/// a channel, and this one has nothing to finish for: the connection
/// ending takes it.
pub async fn notifications(proxy: &Proxy, scope: &ScopeHandle, channel: u32, request: Sessioned<request::Request>) {
    let session = proxy.sessions.get(&request.session).await;
    let mut receiver = session.subscribe();
    if let Err(error) = session.client().await {
        reply(scope, channel, encoded(&response::Frame::Error(error))).await;
        return;
    }
    loop {
        match receiver.recv().await {
            Ok(notification) => {
                let Some(payload) = encoded(&response::Frame::Notification(notification)) else {
                    continue;
                };
                scope.send_channel_response(channel, &payload).await;
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            // The sender lives as long as the proxy: unreachable, and
            // the finish is the honest answer if it ever were.
            Err(broadcast::error::RecvError::Closed) => {
                scope.send_channel_response_finish(channel).await;
                return;
            }
        }
    }
}
