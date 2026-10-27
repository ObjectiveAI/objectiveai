//! Recording an occurrence.

use diverge_sdk::daemon::creator::Creator;
use sqlx::PgConnection;

use super::party::resolve;
use crate::store::Error;

/// Record that something passed from `sender` to `receiver`: the edge
/// added with a count of `1` where the graph had none, else its count
/// raised by one. The unique constraint over the two ends is what
/// makes the choice, so two occurrences racing each other add one
/// edge and count both; the row is the caller's connection's,
/// committed as it commits. Whether an edge was recorded: not when
/// either end's row is gone, since an edge goes with its parties.
pub async fn record(conn: &mut PgConnection, sender: &Creator, receiver: &Creator) -> Result<bool, Error> {
    let Some(sender) = resolve(conn, sender).await? else {
        return Ok(false);
    };
    let Some(receiver) = resolve(conn, receiver).await? else {
        return Ok(false);
    };
    sqlx::query(
        "INSERT INTO diverge.events \
         (sender_kind, sender_issuer, sender_subject, sender_agent, sender_tool, sender_dependency, \
          receiver_kind, receiver_issuer, receiver_subject, receiver_agent, receiver_tool, receiver_dependency) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT ON CONSTRAINT events_edge DO UPDATE SET count = diverge.events.count + 1",
    )
    .bind(sender.kind)
    .bind(sender.issuer)
    .bind(sender.subject)
    .bind(sender.agent)
    .bind(sender.tool)
    .bind(sender.dependency)
    .bind(receiver.kind)
    .bind(receiver.issuer)
    .bind(receiver.subject)
    .bind(receiver.agent)
    .bind(receiver.tool)
    .bind(receiver.dependency)
    .execute(&mut *conn)
    .await?;
    Ok(true)
}

