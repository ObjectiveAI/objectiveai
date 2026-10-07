//! The caller's half of a daemon connection, answered from the proxy.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::Run;

/// The caller's half: everything the program sends, relayed from the
/// proxy.
///
/// The caller quotes the id this end announced on
/// [`relay::daemon`](super::super::relay::daemon)'s channel;
/// [`Pairs::take`](super::super::pairs::Pairs::take) turns it back into
/// the proxy's own id, which opens the matching half toward the proxy.
/// What comes back on it is every client frame the program sent, in
/// order, each relayed onto the caller's channel as it arrives; the
/// proxy's finish is the program's socket ended, and this finishes the
/// caller's channel in turn.
///
/// # An id nobody announced is answered, not held
///
/// A `take` that finds nothing — an id this end never announced, or
/// one whose half was taken already — falls through to the finish with
/// nothing before it, which is what the wire means by a channel that
/// could not be served. Nothing waits for an announcement that may
/// never come.
pub(crate) async fn daemon(run: Arc<Run>, channel: u32, caller_id: u32) {
    if let Some(proxy_id) = run.daemons.take(caller_id).await
        && let Ok(mut program) = run.begin.daemon(proxy_id).await
    {
        while let Some(Ok(frame)) = program.next().await {
            let Some(payload) = encoded(&frame.as_frame()) else {
                break;
            };
            run.scope.send_channel_response(channel, &payload).await;
        }
    }
    run.finish(channel).await;
}
