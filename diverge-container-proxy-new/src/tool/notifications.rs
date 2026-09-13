//! What the container's server says on its own account, for as long
//! as the channel lives.

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::mcp::notifications::response;
use tokio::sync::broadcast;

use super::Tool;
use crate::encode::encoded;
use crate::reply::reply;

/// Subscribe, then relay: one frame per notification, for the
/// connection's life. The client is dialled first so a container
/// whose server cannot be reached answers `Error` at once rather than
/// a silence that could mean anything; a subscriber that fell behind
/// loses the oldest and goes on, which the stream does not report — a
/// notification is a nudge, not a record. Only the responder finishes
/// a channel, and this one has nothing to finish for: the connection
/// ending takes it.
pub async fn notifications(tool: &Tool, scope: &ScopeHandle, channel: u32) {
    let mut receiver = tool.subscribe();
    if let Err(error) = tool.client().await {
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
