//! Wake source for `channels logs subscribe`.
//!
//! Two triggers in `channels/schema.sql` fire NOTIFY with the channel
//! id as payload: `channel_messages_inserted` on every new message,
//! `channel_closed` on the open→closed transition. This helper listens
//! on BOTH and returns on the first notification for `target_channel`,
//! so a blocked subscriber wakes whether a message arrived or the
//! channel closed. The caller re-checks pending + state after the
//! wake — no state is inspected here.

use sqlx::postgres::PgListener;

use super::super::{Error, Pool};

/// Block until the next `channel_messages_inserted` OR `channel_closed`
/// notification whose payload equals `target_channel`. Mismatching
/// notifications are silently consumed.
pub async fn wait_for_channel_event(
    pool: &Pool,
    target_channel: &str,
) -> Result<(), Error> {
    let mut listener = PgListener::connect_with(&**pool).await?;
    listener
        .listen_all(["channel_messages_inserted", "channel_closed"])
        .await?;
    loop {
        let notification = listener.recv().await?;
        if notification.payload() == target_channel {
            return Ok(());
        }
        // A different channel — keep listening.
    }
}
