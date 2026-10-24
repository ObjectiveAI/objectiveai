//! Reading one value of a key without taking it.

use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The value of one slot of the key, chosen at random among every
/// slot it has, held or free — a read locks nothing and is held back
/// by nothing; `None` when the key has no slot, or the vault no such
/// key.
pub async fn read_random(conn: &mut PgConnection, key: &str) -> Result<Option<Value>, Error> {
    let row = sqlx::query("SELECT value FROM diverge.vault_slots WHERE key = $1 ORDER BY random() LIMIT 1")
        .bind(key)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let Json(value): Json<Value> = row.try_get("value")?;
    Ok(Some(value))
}
