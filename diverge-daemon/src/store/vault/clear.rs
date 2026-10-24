//! Every slot freed: what boot does.

use sqlx::PgConnection;

use crate::store::Error;

/// Every held slot freed, its value as it is, and how many were: no
/// run survives a restart, so no hold does either, and a value the
/// run that held it never wrote back is the slot's still.
pub async fn clear_holders(conn: &mut PgConnection) -> Result<u64, Error> {
    let done = sqlx::query("UPDATE diverge.vault_slots SET holder = NULL WHERE holder IS NOT NULL")
        .execute(&mut *conn)
        .await?;
    Ok(done.rows_affected())
}
