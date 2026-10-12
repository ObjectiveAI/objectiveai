//! Loading services.

use diverge_sdk::shared::containers::dependencies::Scope;
use sqlx::PgConnection;

use super::row::{self, ORDER, SELECT};
use super::{Service, scope_name};
use crate::store::{AgentId, Error, ServiceId};

/// Every service, oldest first.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Service>, Error> {
    let rows = sqlx::query(&format!("{SELECT} {ORDER}")).fetch_all(&mut *conn).await?;
    rows.iter().map(row::service).collect()
}

/// The service with the id, if any. With `lock`, its row is locked
/// for the transaction.
pub async fn by_id(conn: &mut PgConnection, id: ServiceId, lock: bool) -> Result<Option<Service>, Error> {
    let locking = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!("{SELECT} WHERE id = $1{locking}"))
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(row::service).transpose()
}

/// The service under the scope key, if any: the template, the scope,
/// and the owner the scope names — the agent, the agent template, or
/// neither.
pub async fn by_scope(
    conn: &mut PgConnection,
    template: &str,
    scope: Scope,
    owner_agent: Option<AgentId>,
    owner_template: Option<&str>,
) -> Result<Option<Service>, Error> {
    let row = sqlx::query(&format!(
        "{SELECT} WHERE template = $1 AND scope = $2 AND owner_agent IS NOT DISTINCT FROM $3 AND owner_template IS NOT DISTINCT FROM $4"
    ))
    .bind(template)
    .bind(scope_name(scope))
    .bind(owner_agent.map(|agent| agent.0))
    .bind(owner_template)
    .fetch_optional(&mut *conn)
    .await?;
    row.as_ref().map(row::service).transpose()
}

/// Every service the agent is attached to, in attach order.
pub async fn of_agent(conn: &mut PgConnection, agent: AgentId) -> Result<Vec<Service>, Error> {
    let rows = sqlx::query(&format!(
        "{SELECT} WHERE id IN (SELECT service FROM diverge.service_attachments WHERE agent = $1) {ORDER}"
    ))
    .bind(agent.0)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(row::service).collect()
}

/// Every service whose scope is per agent template, for the agent
/// template.
pub async fn of_owner_template(conn: &mut PgConnection, template: &str) -> Result<Vec<Service>, Error> {
    let rows = sqlx::query(&format!("{SELECT} WHERE owner_template = $1 {ORDER}"))
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(row::service).collect()
}
