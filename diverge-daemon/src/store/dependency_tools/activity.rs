//! When a dependency tool last changed: deployed, and active.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::store::{DependencyToolId, Error};

/// Keep that its container came up or went down at `when`: what a
/// list reports as its `last_deployed`.
pub async fn set_deployed(conn: &mut PgConnection, id: DependencyToolId, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.dependency_tools SET last_deployed = $2 WHERE id = $1")
        .bind(id.0)
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Keep that it began or ceased working at `when`: what a list reports
/// as its `last_active`.
pub async fn set_active(conn: &mut PgConnection, id: DependencyToolId, when: DateTime<Utc>) -> Result<(), Error> {
    sqlx::query("UPDATE diverge.dependency_tools SET last_active = $2 WHERE id = $1")
        .bind(id.0)
        .bind(when)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
