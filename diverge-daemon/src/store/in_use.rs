//! What containers hold: the templates they were made from, the
//! accounts their tools run under, the remote daemons their tools are
//! on.
//!
//! A template is in use while a container made from it exists; an
//! account while a tool names it; a remote daemon while a connected
//! tool names it; a vault key while an agent or a tool binds it in its
//! `secrets`. All are facts of the records, not of
//! what runs, so they survive a restart as the records do.

use std::collections::HashSet;

use sqlx::{PgConnection, Row as _};

use crate::store::{AccountId, Error};

/// The ids of every agent template some agent was made from.
pub async fn agents_templates(conn: &mut PgConnection) -> Result<HashSet<String>, Error> {
    ids(conn, "SELECT DISTINCT template AS id FROM diverge.agents").await
}

/// The ids of every tool template some created tool was made from.
pub async fn tools_templates(conn: &mut PgConnection) -> Result<HashSet<String>, Error> {
    ids(conn, "SELECT DISTINCT template AS id FROM diverge.tools WHERE kind = 'created'").await
}

/// Whether some tool runs under the account. An agent holds none.
pub async fn account(conn: &mut PgConnection, id: AccountId) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE account = $1) AS held")
    .bind(id.0)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("held")?)
}

/// Whether some connected tool names the remote daemon of the name.
pub async fn daemon(conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE kind = 'connected' AND connected_daemon = $1) AS held")
        .bind(name)
        .fetch_one(&mut *conn)
        .await?;
    Ok(row.try_get("held")?)
}

/// The `id` column of every row of `query`.
async fn ids(conn: &mut PgConnection, query: &str) -> Result<HashSet<String>, Error> {
    let rows = sqlx::query(query).fetch_all(&mut *conn).await?;
    let mut ids = HashSet::with_capacity(rows.len());
    for row in &rows {
        let id: Option<String> = row.try_get("id")?;
        ids.insert(id.unwrap_or_default());
    }
    Ok(ids)
}

/// Whether some agent or tool binds the vault key: a value of its
/// `secrets` is the key.
pub async fn vault_key(conn: &mut PgConnection, key: &str) -> Result<bool, Error> {
    let row = sqlx::query(
        "SELECT EXISTS (SELECT 1 FROM diverge.agents a, jsonb_each_text(a.secrets) s WHERE s.value = $1) \
         OR EXISTS (SELECT 1 FROM diverge.tools t, jsonb_each_text(t.secrets) s WHERE s.value = $1) AS held",
    )
    .bind(key)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("held")?)
}
