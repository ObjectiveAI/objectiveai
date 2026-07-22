//! `logs_messages_inserted` channel subscriber.
//!
//! The trigger in the root `db/schema.sql` fires NOTIFY on every new
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

/// Current most-recent agent-completion `total_tokens` snapshot for an
/// AIH (`objectiveai.agent_token_usage`). `None` when no usage has been
/// recorded for that AIH yet.
pub async fn get_agent_token_usage(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Option<i64>, Error> {
    let total: Option<i64> = sqlx::query_scalar(
        "SELECT total_tokens FROM objectiveai.agent_token_usage \
         WHERE agent_instance_hierarchy = $1",
    )
    .bind(agent_instance_hierarchy)
    .fetch_optional(&**pool)
    .await?;
    Ok(total)
}

/// Block until this AIH's stored `total_tokens` differs from
/// `baseline`, returning the new value.
///
/// Attaches the `agent_token_usage_changed` listener BEFORE the first
/// read, so a write that lands between the caller's baseline read and
/// this call is still observed (the post-attach re-read catches it) —
/// no lost wakeup. A real change is always `Some` (rows are never
/// deleted), so any difference from `baseline` yields the new value;
/// same-value overwrites (the writer's upsert fires the trigger even
/// when the number is unchanged) compare equal and keep waiting.
pub async fn wait_for_token_usage_change(
    pool: &Pool,
    target_aih: &str,
    baseline: Option<i64>,
) -> Result<i64, Error> {
    let mut listener = PgListener::connect_with(&**pool).await?;
    listener.listen("agent_token_usage_changed").await?;
    loop {
        if let Some(total) = get_agent_token_usage(pool, target_aih).await? {
            if Some(total) != baseline {
                return Ok(total);
            }
        }
        // No change yet — wait for the next notification for our AIH.
        loop {
            let notification = listener.recv().await?;
            if notification.payload() == target_aih {
                break;
            }
        }
    }
}
