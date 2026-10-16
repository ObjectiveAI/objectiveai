//! The values a run start hands its container.

use std::collections::HashMap;

use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The value of every one of `keys` the vault holds, by key; a key it
/// does not hold is absent, and the caller says so.
pub async fn values(conn: &mut PgConnection, keys: &[String]) -> Result<HashMap<String, Value>, Error> {
    let rows = sqlx::query("SELECT key, value FROM diverge.vault WHERE key = ANY($1)")
        .bind(keys)
        .fetch_all(&mut *conn)
        .await?;
    let mut values = HashMap::with_capacity(rows.len());
    for row in &rows {
        let key: String = row.try_get("key")?;
        let Json(value): Json<Value> = row.try_get("value")?;
        values.insert(key, value);
    }
    Ok(values)
}
