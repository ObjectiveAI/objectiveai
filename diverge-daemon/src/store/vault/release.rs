//! Freeing a slot a holder holds, its value as it is.

use diverge_sdk::daemon::creator::Creator;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// The slot freed, its value unchanged, where `holder` holds it:
/// whether it did. A slot `holder` does not hold is left as it is.
pub async fn release(conn: &mut PgConnection, key: &str, slot: u32, holder: &Creator) -> Result<bool, Error> {
    let done = sqlx::query("UPDATE diverge.vault_slots SET holder = NULL WHERE key = $1 AND slot = $2 AND holder = $3")
        .bind(key)
        .bind(i32::try_from(slot).unwrap_or(i32::MAX))
        .bind(Json(holder))
        .execute(&mut *conn)
        .await?;
    Ok(done.rows_affected() > 0)
}
