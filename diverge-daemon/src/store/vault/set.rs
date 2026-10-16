//! Setting a vault key.

use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// What a set came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Set {
    /// The key, made, holding the value.
    Created,
    /// The key the vault held, its value replaced.
    Updated,
}

/// Make the key hold `value`: inserted with `creator` where the vault
/// had no such key, its value replaced where it had — tags, the time
/// and the maker as they were.
pub async fn set(conn: &mut PgConnection, key: &str, value: &Value, creator: &Creator) -> Result<Set, Error> {
    let row = sqlx::query(
        "INSERT INTO diverge.vault (key, value, creator) VALUES ($1, $2, $3) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value RETURNING (xmax = 0) AS inserted",
    )
    .bind(key)
    .bind(Json(value))
    .bind(Json(creator))
    .fetch_one(&mut *conn)
    .await?;
    let inserted: bool = row.try_get("inserted")?;
    Ok(if inserted { Set::Created } else { Set::Updated })
}
