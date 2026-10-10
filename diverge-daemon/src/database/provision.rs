//! The statements a scope is: made, and dropped.
//!
//! Run on the daemon's own connection, as the URL's role, which holds
//! `CREATEROLE` and `CREATE` on the database and nothing more. The
//! role name is structurally `[a-z0-9_]`, so it is written bare; the
//! verifier is a quoted literal. Each statement is one `query`, and
//! the caller's transaction holds them together under
//! [`lock`](crate::database::lock_key).

use postgres_protocol::escape::escape_literal;
use postgres_protocol::password::scram_sha_256;
use sqlx::{PgConnection, Row as _};

use super::lock_key;
use crate::store;

/// Whether the daemon's role may provision a scope here: `CREATEROLE`
/// or superuser, and `CREATE` on the database. The lack, in a
/// sentence, when it may not.
pub async fn capable(conn: &mut PgConnection) -> Result<Result<(), String>, store::Error> {
    let row = sqlx::query(
        "SELECT (rolcreaterole OR rolsuper) AS roles, has_database_privilege(current_database(), 'CREATE') AS create \
         FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else {
        return Ok(Err("the daemon's role is not in pg_roles".to_string()));
    };
    let roles: bool = row.try_get("roles")?;
    let create: bool = row.try_get("create")?;
    Ok(match (roles, create) {
        (true, true) => Ok(()),
        (false, _) => Err("the daemon's role holds no CREATEROLE, so it cannot make a container's role".to_string()),
        (_, false) => Err("the daemon's role holds no CREATE on the database, so it cannot make a container's schema".to_string()),
    })
}

/// Make the scope, or find it made: the role, a member of nothing and
/// able to create nowhere but its schema, given the SCRAM-SHA-256
/// verifier of `password`; the schema, owned by the role; the search
/// path pinned to the schema alone. Idempotent, and serialized against
/// another connection making the same scope by an advisory lock on
/// the role, held for the caller's transaction.
pub async fn ensure(conn: &mut PgConnection, role: &str, password: &str) -> Result<(), store::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_key(role))
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(&mut *conn)
    .await?;
    let verifier = escape_literal(&scram_sha_256(password.as_bytes()));
    sqlx::query(&format!("ALTER ROLE {role} PASSWORD {verifier}"))
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {role} AUTHORIZATION {role}"))
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("ALTER ROLE {role} SET search_path = {role}"))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Drop the scope: every session of the role ended, the schema and
/// everything in it, then the role. A role that owns something in
/// another database of the cluster cannot be dropped and is left,
/// owning nothing here and holding no password anyone knows; the
/// schema is gone either way.
pub async fn drop(conn: &mut PgConnection, role: &str) -> Result<(), store::Error> {
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE usename = $1 AND pid <> pg_backend_pid()")
        .bind(role)
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {role} CASCADE"))
        .execute(&mut *conn)
        .await?;
    let _ = sqlx::query(&format!("DROP ROLE IF EXISTS {role}")).execute(&mut *conn).await;
    Ok(())
}

/// Drop every scope under the prefix: what an owner has — a
/// container's own scope and, an agent's, every scope of its
/// `per_agent_instance` dependencies; an agent template's, every
/// `per_agent_template` scope its agents' dependencies shared, swept
/// at the template's delete once no agent is left of it — found in
/// the catalog by the owner's prefix, which every role of the owner's
/// begins with, and [`drop`]ped one by one. A `per_agent_template`
/// scope is under no agent's prefix; a tool template owns none.
pub async fn sweep(conn: &mut PgConnection, prefix: &str) -> Result<(), store::Error> {
    let pattern = format!("{}%", prefix.replace('_', "\\_"));
    let rows = sqlx::query(r"SELECT nspname FROM pg_namespace WHERE nspname LIKE $1 ESCAPE '\'")
        .bind(&pattern)
        .fetch_all(&mut *conn)
        .await?;
    for row in &rows {
        let role: String = row.try_get("nspname")?;
        drop(&mut *conn, &role).await?;
    }
    Ok(())
}

