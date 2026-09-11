//! The caller's half of a database connection.

use std::sync::Arc;

use super::super::run::Run;

/// What the container's driver wrote, as this channel's responses,
/// until the driver's socket ends; then the finish. The queue was
/// filling from the moment the pair was asked for — see
/// [`relay::postgres`](super::super::relay) — so nothing the driver
/// said before the caller dialled is lost. An id this end did not
/// mint, or a half already taken, is the finish with nothing.
pub(crate) async fn postgres(run: Arc<Run>, channel: u32, connection_id: u32) {
    if let Some(mut from_container) = run.pairs.take(connection_id) {
        while let Some(bytes) = from_container.recv().await {
            run.scope.send_channel_response(channel, &bytes).await;
        }
    }
    run.finish(channel).await;
}
