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
/// notification sent on it, for as long as the server reads the
/// channel. The rules, in order:
///
/// 1. The session's client is dialled first, so a container whose
///    server cannot be reached answers `Error`, then the finish, at
///    once rather than a silence that could mean anything.
/// 2. Each notification is one frame. A subscriber that fell behind
///    loses the oldest and goes on, which the stream does not report
///    — a notification is a nudge, not a record.
/// 3. The server reading no more of the channel — it abandoned the
///    answer, or the scope or the connection ended — is the finish,
///    and the task ends: nothing waits on a broadcast nobody hears.
/// 4. The broadcast closing is the finish too. Its sender lives as
///    long as the session, so this is unreachable while it is held.
pub async fn notifications(proxy: &Proxy, scope: &ScopeHandle, channel: u32, request: Sessioned<request::Request>) {
    let session = proxy.sessions.get(&request.session).await;
    let mut receiver = session.subscribe();
    if let Err(error) = session.client().await {
        reply(scope, channel, encoded(&response::Frame::Error(error))).await;
        return;
    }
    loop {
        let received = tokio::select! {
            received = receiver.recv() => received,
            () = scope.answer_gone(channel) => break,
        };
        match received {
            Ok(notification) => {
                let Some(payload) = encoded(&response::Frame::Notification(notification)) else {
                    continue;
                };
                scope.send_channel_response(channel, &payload).await;
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    scope.send_channel_response_finish(channel).await;
}
