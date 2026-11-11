//! Changing an account's own columns.

use sqlx::PgConnection;

use crate::store::{AccountId, Error};

/// An account's own columns, as they are to be: the whole of them,
/// since an edit is applied whole. The identity is not among them: an
/// account is its identity, and another identity is another account.
#[derive(Debug, Clone)]
pub struct Columns {
    /// What the account is for.
    pub description: Option<String>,
}

/// Write the columns.
pub async fn update(conn: &mut PgConnection, id: AccountId, columns: &Columns) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.accounts SET description = $2 WHERE id = $1")
        .bind(id.0)
        .bind(&columns.description)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
