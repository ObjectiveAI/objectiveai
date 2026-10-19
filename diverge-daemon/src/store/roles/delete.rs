//! Removing a role.

use sqlx::PgConnection;

use crate::store::{Error, RoleId};

/// What a delete came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deleted {
    /// The role, gone.
    Deleted,
    /// An account holds the role, which the database refuses to drop
    /// from under it; nothing changed.
    InUse,
}

/// Delete the role, unless an account holds it.
pub async fn delete(conn: &mut PgConnection, id: RoleId) -> Result<Deleted, Error> {
    let deleted = sqlx::query("DELETE FROM diverge.roles WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await;
    match deleted {
        Ok(_) => Ok(Deleted::Deleted),
        Err(sqlx::Error::Database(error)) if error.is_foreign_key_violation() => Ok(Deleted::InUse),
        Err(error) => Err(error.into()),
    }
}
