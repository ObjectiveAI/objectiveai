//! The DDL, applied at every start.

use sqlx::PgConnection;

use super::Error;

/// The schema, as `schema.sql` beside this file states it.
const SCHEMA: &str = include_str!("schema.sql");

/// Apply the schema on `conn`. Every statement is idempotent, so a
/// database that has the schema is left as it is.
/// Nothing here brings an older database forward: the schema is
/// made whole on a fresh database, and a database an earlier shape
/// of it made is not carried over.
pub async fn apply(conn: &mut PgConnection) -> Result<(), Error> {
    sqlx::raw_sql(SCHEMA).execute(&mut *conn).await?;
    Ok(())
}
