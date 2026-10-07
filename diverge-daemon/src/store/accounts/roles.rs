//! Which roles an account holds.

use sqlx::PgConnection;

use crate::store::{AccountId, Error, RoleId};

/// Make the account hold exactly `roles`: what it held is dropped and
/// the list written, whole.
pub async fn set_roles(conn: &mut PgConnection, id: AccountId, roles: &[RoleId]) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.account_roles WHERE account = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    let ids: Vec<i64> = roles.iter().map(|role| role.0).collect();
    sqlx::query("INSERT INTO diverge.account_roles (account, role) SELECT $1, unnest($2::bigint[]) ON CONFLICT DO NOTHING")
        .bind(id.0)
        .bind(&ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
