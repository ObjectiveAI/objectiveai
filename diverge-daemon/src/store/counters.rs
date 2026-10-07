//! The once-and-for-all index of a container.

use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The next index for `key` — one key per way of making a container:
/// `agents:<template>`, `tools:<template>`, or
/// `tools:<provider JSON>:<id>` for a connected tool. The first is
/// `1`, and no number is given twice: the row is advanced in the
/// caller's transaction, so a create that does not commit gives its
/// number back.
pub async fn next(conn: &mut PgConnection, key: &str) -> Result<u64, Error> {
    let row = sqlx::query(
        "INSERT INTO diverge.counters (key, next) VALUES ($1, 1) \
         ON CONFLICT (key) DO UPDATE SET next = diverge.counters.next + 1 RETURNING next",
    )
    .bind(key)
    .fetch_one(&mut *conn)
    .await?;
    let next: i64 = row.try_get("next")?;
    Ok(u64::try_from(next).unwrap_or(0))
}
