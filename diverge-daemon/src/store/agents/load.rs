//! Loading agents.

use diverge_sdk::daemon::reference;
use sqlx::{PgConnection, Row as _};

use super::Agent;
use super::row::{self, ORDER, SELECT};
use crate::store::{AgentId, Error};

/// Every agent, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Agent>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::agent).collect()
}

/// The agent with the id, if any. With `lock`, its row is locked for
/// the transaction.
pub async fn by_id(conn: &mut PgConnection, id: AgentId, lock: bool) -> Result<Option<Agent>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE id = $1{locking}"))
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::agent).transpose()
}

/// The agent the reference names, if any — by name, or by template
/// and index. With `lock`, its row is locked for the transaction, so
/// that two edits of one agent take turns.
pub async fn by_reference(conn: &mut PgConnection, reference: &reference::Agent, lock: bool) -> Result<Option<Agent>, Error> {
    let found = match reference {
        reference::Agent::Name { name } => {
            sqlx::query("SELECT id FROM diverge.agents WHERE name = $1")
                .bind(name)
                .fetch_optional(&mut *conn)
                .await?
        }
        reference::Agent::TemplateIndex { template, index } => {
            sqlx::query("SELECT id FROM diverge.agents WHERE template = $1 AND index = $2")
                .bind(template)
                .bind(i64::try_from(*index).unwrap_or(i64::MAX))
                .fetch_optional(&mut *conn)
                .await?
        }
    };
    let Some(found) = found else {
        return Ok(None);
    };
    by_id(conn, AgentId(found.try_get("id")?), lock).await
}
