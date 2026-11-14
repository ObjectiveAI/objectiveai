//! What containers hold: the templates they were made from, the remote
//! daemons their tools are on.
//!
//! A template is in use while a container made from it exists; a
//! remote daemon while a connected
//! tool names it, or a mount of a volume on it; a vault key while an agent or a tool binds it in its
//! `secrets`. All are facts of the records, not of
//! what runs, so they survive a restart as the records do.

use std::collections::HashSet;

use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, daemons};

/// The ids of every agent template some agent was made from.
pub async fn agents_templates(conn: &mut PgConnection) -> Result<HashSet<String>, Error> {
    ids(conn, "SELECT DISTINCT template AS id FROM diverge.agents").await
}

/// The ids of every tool template some standalone tool was made from.
pub async fn tools_templates(conn: &mut PgConnection) -> Result<HashSet<String>, Error> {
    ids(conn, "SELECT DISTINCT template AS id FROM diverge.tools").await
}

/// Whether something names the remote daemon's record of the ID as the
/// daemon it is on: a connected tool, or a FUSE mount of an agent's or
/// a tool's whose volume is on that daemon. What refuses the record's
/// delete. A record named only further along a route is not held by
/// it — that record is the other daemon's to keep.
pub async fn daemon(conn: &mut PgConnection, id: DaemonId) -> Result<bool, Error> {
    let mounts = serde_json::json!([{ "daemon": id }]);
    let row = sqlx::query(
        "SELECT EXISTS (SELECT 1 FROM diverge.connected_standalone_tools WHERE daemon = $1) \
         OR EXISTS (SELECT 1 FROM diverge.agents WHERE fuse_file_mounts @> $2 OR fuse_directory_mounts @> $2) \
         OR EXISTS (SELECT 1 FROM diverge.tools WHERE fuse_file_mounts @> $2 OR fuse_directory_mounts @> $2) AS held",
    )
    .bind(daemons::to_column(id))
    .bind(Json(&mounts))
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
