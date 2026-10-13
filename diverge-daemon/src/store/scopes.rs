//! Every database scope the daemon has provisioned: its role, the
//! dependency template it was made for, and whose it is.
//!
//! A scope is made at a container's first connection and dropped by
//! exactly one delete — a tool's own by the tool's, a dependency
//! tool's by its template's — and nothing else drops one; this is
//! how a template's delete finds every scope ever made from it, under
//! whatever owner. A row is written when the scope is made and taken
//! off when it is dropped.

use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The scope is made: its role, the dependency template it was made
/// for a tool of — `None` for a container's own scope — and whose it
/// is, the daemon's [`Owner`](crate::database::Owner) as JSON. One
/// recorded already stays as it was.
pub async fn record(conn: &mut PgConnection, role: &str, template: Option<&str>, owner: &serde_json::Value) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.scopes (role, template, owner) VALUES ($1, $2, $3) ON CONFLICT (role) DO NOTHING")
        .bind(role)
        .bind(template)
        .bind(Json(owner))
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Every scope made for a tool of the dependency template, by role.
pub async fn of_template(conn: &mut PgConnection, template: &str) -> Result<Vec<String>, Error> {
    let rows = sqlx::query("SELECT role FROM diverge.scopes WHERE template = $1 ORDER BY role")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| Ok(row.try_get("role")?)).collect()
}

/// The scope is dropped: its row taken off.
pub async fn remove(conn: &mut PgConnection, role: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.scopes WHERE role = $1")
        .bind(role)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
