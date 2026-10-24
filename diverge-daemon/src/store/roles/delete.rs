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

/// The SQLSTATE PostgreSQL reports a delete refused by `ON DELETE
/// RESTRICT` under: `restrict_violation`, which is not the
/// `foreign_key_violation` (`23503`) sqlx knows as one, so it is
/// matched by its code.
const RESTRICT_VIOLATION: &str = "23001";

/// Delete the role, unless an account holds it: the `ON DELETE
/// RESTRICT` of the holding's reference refuses the delete, and the
/// refusal — a restrict violation, or a foreign key violation where
/// the database reports it so — is the role in use.
pub async fn delete(conn: &mut PgConnection, id: RoleId) -> Result<Deleted, Error> {
    let deleted = sqlx::query("DELETE FROM diverge.roles WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await;
    match deleted {
        Ok(_) => Ok(Deleted::Deleted),
        Err(sqlx::Error::Database(error)) if error.is_foreign_key_violation() || error.code().as_deref() == Some(RESTRICT_VIOLATION) => {
            Ok(Deleted::InUse)
        }
        Err(error) => Err(error.into()),
    }
}
