//! `logs_messages_inserted` channel subscriber.
//!
//! The trigger in `logs/schema.sql` fires NOTIFY on every new
//! `objectiveai.messages` row with `NEW.agent_instance_hierarchy` as the
//! payload. This helper attaches a `PgListener`, drops every
//! notification whose payload doesn't equal `target_aih`, and
//! returns `Ok(())` on the first match. The caller is expected to
//! re-issue `read_pending_for_parent` after the match — type
//! filtering happens there, not here.
//!
//! No belt-and-suspenders pre-check (cf. `subscribe_delivered`):
//! we're waiting for a FUTURE row, not for a flag flip on a
//! known row. There's no observable state to inspect before the
//! LISTEN attaches.

use sqlx::postgres::PgListener;

use super::super::{Error, Pool};

/// Block until the next `objectiveai.messages` INSERT whose
/// `agent_instance_hierarchy` payload equals `target_aih`.
/// Mismatching notifications are silently consumed.
pub async fn wait_for_logs_message_at(
    pool: &Pool,
    target_aih: &str,
) -> Result<(), Error> {
    let mut listener = PgListener::connect_with(&**pool).await?;
    listener.listen("logs_messages_inserted").await?;
    loop {
        let notification = listener.recv().await?;
        if notification.payload() == target_aih {
            return Ok(());
        }
        // Different AIH — keep listening.
    }
}
