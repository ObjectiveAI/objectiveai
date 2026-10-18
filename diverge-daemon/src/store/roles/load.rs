//! Loading roles.

use sqlx::PgConnection;

use super::row::{self, ORDER, SELECT};
use super::Role;
use crate::store::Error;

/// Every role, oldest created first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Role>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    row::roles(conn, &rows).await
}

/// The role with the name, if any. With `lock`, its row is locked for
/// the transaction.
pub async fn by_name(conn: &mut PgConnection, name: &str, lock: bool) -> Result<Option<Role>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let rows = sqlx::query(&format!("{SELECT} WHERE name = $1{locking}"))
        .bind(name)
        .fetch_all(&mut *conn)
        .await?;
    Ok(row::roles(conn, &rows).await?.pop())
}

/// The roles with these names, in the order of the names; a name
/// that is no role's is simply absent, and the caller counts.
pub async fn by_names(conn: &mut PgConnection, names: &[String]) -> Result<Vec<Role>, Error> {
    let rows = sqlx::query(&format!("{SELECT} WHERE name = ANY($1) {ORDER}"))
        .bind(names)
        .fetch_all(&mut *conn)
        .await?;
    let mut roles = row::roles(conn, &rows).await?;
    roles.sort_by_key(|role| names.iter().position(|name| *name == role.name));
    Ok(roles)
}
