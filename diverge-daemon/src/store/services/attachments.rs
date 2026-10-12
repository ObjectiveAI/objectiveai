//! Agents attached to services, in the order they attached.
//!
//! An agent is attached to a service by a registration that declares
//! the service's template, and detached by one that no longer does;
//! what holds a service up is that some agent is attached to it.

use sqlx::{PgConnection, Row as _};

use crate::store::{AgentId, Error, ServiceId};

/// One attachment: the service, the agent, and its place in the
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    /// The service.
    pub service: ServiceId,
    /// The agent attached to it.
    pub agent: AgentId,
    /// The order of attaching, across every service and agent.
    pub seq: i64,
}

/// Every attachment, in attach order: what a listing folds in, loaded
/// once for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Attachment>, Error> {
    let rows = sqlx::query("SELECT service, agent, seq FROM diverge.service_attachments ORDER BY seq")
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(attachment).collect()
}

/// The agents attached to the service, in attach order.
pub async fn attached(conn: &mut PgConnection, service: ServiceId) -> Result<Vec<AgentId>, Error> {
    let rows = sqlx::query("SELECT service, agent, seq FROM diverge.service_attachments WHERE service = $1 ORDER BY seq")
        .bind(service.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| attachment(row).map(|attachment| attachment.agent)).collect()
}

/// Attach the agent to the service. One already attached stays as it
/// was, in its place.
pub async fn attach(conn: &mut PgConnection, service: ServiceId, agent: AgentId) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.service_attachments (service, agent) VALUES ($1, $2) ON CONFLICT (service, agent) DO NOTHING")
        .bind(service.0)
        .bind(agent.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Detach the agent from every service but those in `keep`: what a
/// registration does for the services it no longer declares. The
/// services detached from, for their holds to be told.
pub async fn detach_except(conn: &mut PgConnection, agent: AgentId, keep: &[ServiceId]) -> Result<Vec<ServiceId>, Error> {
    let kept: Vec<i64> = keep.iter().map(|service| service.0).collect();
    let rows = sqlx::query("DELETE FROM diverge.service_attachments WHERE agent = $1 AND NOT (service = ANY($2)) RETURNING service")
        .bind(agent.0)
        .bind(&kept)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| Ok(ServiceId(row.try_get("service")?))).collect()
}

/// The attachment a row holds.
fn attachment(row: &sqlx::postgres::PgRow) -> Result<Attachment, Error> {
    Ok(Attachment {
        service: ServiceId(row.try_get("service")?),
        agent: AgentId(row.try_get("agent")?),
        seq: row.try_get("seq")?,
    })
}
