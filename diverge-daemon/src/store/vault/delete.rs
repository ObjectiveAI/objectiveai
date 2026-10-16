//! Removing a vault key.

use sqlx::PgConnection;

use crate::store::{Error, in_use};

/// What a delete came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleted {
    /// The key, gone.
    Deleted,
    /// An agent or a tool binds the key, which is left as it is;
    /// nothing changed.
    InUse,
}

/// Delete the key, unless a record binds it. The caller holds the
/// key's row locked, so a binding made between the look and the
/// delete waits on the same row the other way: a create or an edit
/// that binds a key reads it under the same transaction first.
pub async fn delete(conn: &mut PgConnection, key: &str) -> Result<Deleted, Error> {
    if in_use::vault_key(conn, key).await? {
        return Ok(Deleted::InUse);
    }
    sqlx::query("DELETE FROM diverge.vault WHERE key = $1")
        .bind(key)
        .execute(&mut *conn)
        .await?;
    Ok(Deleted::Deleted)
}
