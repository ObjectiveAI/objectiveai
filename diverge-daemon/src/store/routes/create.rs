//! Putting a route down.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{Error, ToolId};

/// What a new route is made of: the position, the tool — a created
/// one of the position's last template, which the caller has checked
/// — and who puts it down.
#[derive(Debug, Clone)]
pub struct New {
    /// The position.
    pub path: Path,
    /// The tool that answers there.
    pub tool: ToolId,
    /// Who puts it down.
    pub creator: Creator,
}

/// What putting one down came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The route is down.
    Created,
    /// The position has a route already; nothing changed.
    Exists,
}

/// Insert the route.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.routes (agent, template, tool, creator) VALUES ($1, $2, $3, $4)")
        .bind(&new.path.agent)
        .bind(&new.path.template)
        .bind(new.tool.0)
        .bind(Json(&new.creator))
        .execute(&mut *conn)
        .await;
    match inserted {
        Ok(_) => Ok(Created::Created),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
