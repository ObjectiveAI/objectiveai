//! Putting a holder's value back into the slot it holds.

use diverge_sdk::daemon::creator::Creator;
use serde_json::Value;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// The slot's value replaced with `value` and the slot freed, where
/// `holder` holds it: whether it did. A slot `holder` does not hold —
/// freed at boot, or gone with its key — is left as it is, and the
/// value is not kept anywhere.
pub async fn write_back(conn: &mut PgConnection, key: &str, slot: u32, holder: &Creator, value: &Value) -> Result<bool, Error> {
    let done = sqlx::query("UPDATE diverge.vault_slots SET value = $4, holder = NULL WHERE key = $1 AND slot = $2 AND holder = $3")
        .bind(key)
        .bind(i32::try_from(slot).unwrap_or(i32::MAX))
        .bind(Json(holder))
        .bind(Json(value))
        .execute(&mut *conn)
        .await?;
    Ok(done.rows_affected() > 0)
}
