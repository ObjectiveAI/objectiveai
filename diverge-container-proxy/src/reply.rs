//! One answer on a channel the server opened, then the finish.

use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// The one channel response, when there is one, and the finish
/// either way: a finish with nothing before it is the wire's word for
/// a channel that could not be served.
pub async fn reply(scope: &ScopeHandle, channel: u32, payload: Option<Vec<u8>>) {
    if let Some(payload) = payload {
        scope.send_channel_response(channel, &payload).await;
    }
    scope.send_channel_response_finish(channel).await;
}
