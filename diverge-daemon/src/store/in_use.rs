//! What containers hold: the templates they were made from, the
//! accounts they run under, the daemons their tools are on; and what
//! daemon records hold: the providers their links name.
//!
//! A template is in use while a container made from it exists; an
//! account while a container names it; a daemon record while a
//! connected tool names it; a provider while a daemon record links
//! through it. All are facts of the records, not of what runs, so
//! they survive a restart as the records do.

use std::collections::HashSet;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::types::Json;
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

/// Whether some agent or tool runs under the account.
pub async fn account(conn: &mut PgConnection, id: AccountId) -> Result<bool, Error> {
    let row = sqlx::query(
        "SELECT EXISTS (SELECT 1 FROM diverge.agents WHERE account = $1) \
             OR EXISTS (SELECT 1 FROM diverge.tools WHERE account = $1) AS held",
    )
    .bind(id.0)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row.try_get("held")?)
}

/// Whether some connected tool names the daemon record.
pub async fn daemon(conn: &mut PgConnection, name: &str) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.tools WHERE kind = 'connected' AND connected_daemon = $1) AS held")
        .bind(name)
        .fetch_one(&mut *conn)
        .await?;
    Ok(row.try_get("held")?)
}

/// Whether some daemon record links through the provider: a link
/// naming it, by JSON containment of one element.
pub async fn provider_linked(conn: &mut PgConnection, identity: &Identity) -> Result<bool, Error> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM diverge.providers_daemons WHERE links @> $1) AS held")
        .bind(Json(serde_json::json!([{ "provider": identity }])))
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
