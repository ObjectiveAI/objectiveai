//! Making a tool: from a template, or joined to another daemon's
//! tool.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount;
use diverge_sdk::shared::canonical;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::Origin;
use crate::store::{AccountId, Error, ToolId, counters};

/// What a new tool is made of: its origin, checked by the caller —
/// the template live, the daemon record there — and what a create
/// adds, which a connect adds none of but the name.
#[derive(Debug, Clone)]
pub struct New {
    /// How it comes to be.
    pub origin: Origin,
    /// The name, if any.
    pub name: Option<String>,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// Files of other providers' volumes.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// Who makes it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The tool, made, with the index it was given.
    Created(ToolId, u64),
    /// The name is another tool's already; nothing was made.
    Exists,
}

/// Insert the tool, its index the next among tools made the same way,
/// holding no tag yet. The counter is advanced in the caller's
/// transaction, so a create that is not committed gives the number
/// back.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let key = match &new.origin {
        Origin::Created { template, .. } => format!("tools:{template}"),
        Origin::Connected { daemon, tool } => {
            let tool = canonical::bytes(tool).map_err(Error::Json)?;
            format!("tools:{daemon}:{}", String::from_utf8_lossy(&tool))
        }
    };
    let index = counters::next(conn, &key).await?;
    let (kind, template, provider, connected_daemon, connected_tool) = match &new.origin {
        Origin::Created { template, provider } => ("created", Some(template.clone()), provider.as_ref().map(Json), None, None),
        Origin::Connected { daemon, tool } => ("connected", None, None, Some(daemon.clone()), Some(Json(tool))),
    };
    let inserted = sqlx::query(
        "INSERT INTO diverge.tools (kind, template, provider, connected_daemon, connected_tool, index, name, \
         account, fuse_file_mounts, fuse_directory_mounts, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(kind)
    .bind(template)
    .bind(provider)
    .bind(connected_daemon)
    .bind(connected_tool)
    .bind(i64::try_from(index).unwrap_or(i64::MAX))
    .bind(&new.name)
    .bind(new.account.map(|account| account.0))
    .bind(Json(&new.fuse_file_mounts))
    .bind(Json(&new.fuse_directory_mounts))
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(ToolId(row.try_get("id")?), index)),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
