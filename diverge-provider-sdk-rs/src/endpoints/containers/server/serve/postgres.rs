//! The caller's half of a database connection: this end's half on
//! the begin scope, relayed.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::run::Run;

/// The caller opened its half quoting `caller_id`. Take the proxy's
/// id from under it, open this end's half on the begin scope with
/// that, and put everything the driver wrote on the caller's channel
/// until the proxy finishes — the driver's socket ended. An id this
/// end did not mint, or a half already taken, is the finish with
/// nothing.
pub(crate) async fn postgres(run: Arc<Run>, channel: u32, caller_id: u32) {
    if let Some(proxy_id) = run.pairs.take(caller_id)
        && let Ok(mut driver) = run.begin.postgres(proxy_id).await
    {
        while let Some(Ok(bytes)) = driver.next().await {
            run.scope.send_channel_response(channel, &bytes).await;
        }
    }
    run.finish(channel).await;
}
