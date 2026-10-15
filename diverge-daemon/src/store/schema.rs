//! The DDL, applied at every start.

use sqlx::PgConnection;

use super::Error;

/// The schema, as `schema.sql` beside this file states it.
const SCHEMA: &str = include_str!("schema.sql");

/// Apply the schema on `conn`, and answer whether the database was
/// FRESH — had no `diverge.accounts` table before this — which is
/// what decides whether root is seeded. Every statement is
/// idempotent, so a database that has the schema is left as it is.
pub async fn apply(conn: &mut PgConnection) -> Result<bool, Error> {
    let fresh: bool = sqlx::query_scalar("SELECT to_regclass('diverge.accounts') IS NULL")
        .fetch_one(&mut *conn)
        .await?;
    sqlx::raw_sql(SCHEMA).execute(&mut *conn).await?;
    Ok(fresh)
}
