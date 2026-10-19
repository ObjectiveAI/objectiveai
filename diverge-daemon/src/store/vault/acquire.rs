//! Taking one free value of a key for a running container.

use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// One free slot of the key taken for `holder`, chosen at random
/// among the slots nobody holds, and the slot's number and value;
/// `None` when the key has no free slot — none at all, every one
/// held, or every free one being taken by another transaction this
/// instant, which `SKIP LOCKED` passes over rather than waits on. The
/// hold is the caller's transaction's until it commits, and the
/// record's from then on.
pub async fn acquire(conn: &mut PgConnection, key: &str, holder: &Creator) -> Result<Option<(u32, Value)>, Error> {
    let row = sqlx::query(
        "UPDATE diverge.vault_slots SET holder = $2 \
         WHERE key = $1 AND slot = ( \
             SELECT slot FROM diverge.vault_slots WHERE key = $1 AND holder IS NULL \
             ORDER BY random() LIMIT 1 FOR UPDATE SKIP LOCKED \
         ) \
         RETURNING slot, value",
    )
    .bind(key)
    .bind(Json(holder))
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let slot: i32 = row.try_get("slot")?;
    let Json(value): Json<Value> = row.try_get("value")?;
    Ok(Some((u32::try_from(slot).unwrap_or(0), value)))
}
