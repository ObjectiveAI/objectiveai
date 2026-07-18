//! Wake source for `channels logs subscribe`.
//!
//! Two triggers in `channels/schema.sql` fire NOTIFY with the channel
//! id as payload: `channel_messages_inserted` on every new message,
//! `channel_closed` on the open→closed transition. A subscriber
//! attaches [`channel_event_listener`] BEFORE its pending/state check,
//! then waits with [`recv_channel_event`] — so a message or close that
//! lands between the check and the wait is buffered by the already-
//! attached listener and returned immediately (no lost wakeup). The
//! caller re-checks pending + state after each wake; no state is
//! inspected here.

use sqlx::postgres::PgListener;

use super::super::{Error, Pool};

/// Attach a listener on both channel-event NOTIFY channels. Attach it
/// BEFORE the caller's first pending/state check to avoid a lost
/// wakeup.
pub async fn channel_event_listener(pool: &Pool) -> Result<PgListener, Error> {
    let mut listener = PgListener::connect_with(&**pool).await?;
    listener
        .listen_all(["channel_messages_inserted", "channel_closed"])
        .await?;
    Ok(listener)
}

/// Block on `listener` until the next notification whose payload equals
/// `target_channel` (a new message OR a close). Notifications for other
/// channels are silently consumed.
pub async fn recv_channel_event(
    listener: &mut PgListener,
    target_channel: &str,
) -> Result<(), Error> {
    loop {
        let notification = listener.recv().await?;
        if notification.payload() == target_channel {
            return Ok(());
        }
    }
}
