//! Removing a service.

use sqlx::{PgConnection, Row as _};

use crate::store::{Error, ServiceId};

/// Delete the service; its attachments go with it. That its run is
/// stopped is the caller's: the run is nowhere in the row. One
/// already gone is nothing to do.
pub async fn delete(conn: &mut PgConnection, id: ServiceId) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.services WHERE id = $1")
        .bind(id.0)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete every service whose scope is per agent template, for the
/// agent template: what the template's delete does, no agent of the
/// template being left to be attached. The ids removed, for their
/// runs to be stopped.
pub async fn delete_of_owner_template(conn: &mut PgConnection, template: &str) -> Result<Vec<ServiceId>, Error> {
    let rows = sqlx::query("DELETE FROM diverge.services WHERE owner_template = $1 RETURNING id")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| Ok(ServiceId(row.try_get("id")?))).collect()
}

/// Delete every service made from the dependency template, under
/// whatever scope: what the template's delete does. The ids removed,
/// for their states to be retired.
pub async fn delete_of_template(conn: &mut PgConnection, template: &str) -> Result<Vec<ServiceId>, Error> {
    let rows = sqlx::query("DELETE FROM diverge.services WHERE template = $1 RETURNING id")
        .bind(template)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(|row| Ok(ServiceId(row.try_get("id")?))).collect()
}
