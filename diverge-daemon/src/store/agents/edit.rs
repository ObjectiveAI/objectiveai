//! Changing what an edit may change about an agent.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::{AccountId, AgentId, Error};

/// An agent's editable columns, as they are to be: the whole of them,
/// since an edit is applied whole. The caller computes them from the
/// agent as loaded and the changes asked for. The provider pin never
/// changes; its volume list does, so the provider is here whole.
#[derive(Debug, Clone)]
pub struct Columns {
    /// The name, if any.
    pub name: Option<String>,
    /// The account, if any.
    pub account: Option<AccountId>,
    /// The provider pin with its volumes, if any.
    pub provider: Option<Provider>,
    /// Files of other providers' volumes.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes.
    pub fuse_directory_mounts: Vec<FuseMount>,
}

/// What an update came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updated {
    /// The columns are as given.
    Updated,
    /// The name is another agent's already; nothing changed.
    InUse,
}

/// Write the columns.
pub async fn update(conn: &mut PgConnection, id: AgentId, columns: &Columns) -> Result<Updated, Error> {
    let written = sqlx::query(
        "UPDATE diverge.agents SET name = $2, account = $3, provider = $4, fuse_file_mounts = $5, \
         fuse_directory_mounts = $6 WHERE id = $1",
    )
    .bind(id.0)
    .bind(&columns.name)
    .bind(columns.account.map(|account| account.0))
    .bind(columns.provider.as_ref().map(Json))
    .bind(Json(&columns.fuse_file_mounts))
    .bind(Json(&columns.fuse_directory_mounts))
    .execute(&mut *conn)
    .await;
    match written {
        Ok(_) => Ok(Updated::Updated),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Updated::InUse),
        Err(error) => Err(error.into()),
    }
}
