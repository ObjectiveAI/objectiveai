//! Changing a role's own columns.

use diverge_sdk::daemon::grant::Grant;
use sqlx::types::Json;
use sqlx::PgConnection;

use crate::store::{Error, RoleId};

/// A role's own columns, as they are to be: the whole of them. The
/// name is not among them, since a role's name never changes.
#[derive(Debug, Clone)]
pub struct Columns {
    /// What the role is for.
    pub description: Option<String>,
    /// Its grants, whole.
    pub grants: Vec<Grant>,
}

/// Write the columns.
pub async fn update(conn: &mut PgConnection, id: RoleId, columns: &Columns) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.roles SET description = $2, grants = $3 WHERE id = $1")
        .bind(id.0)
        .bind(&columns.description)
        .bind(Json(&columns.grants))
        .execute(&mut *conn)
        .await?;
    Ok(())
}
