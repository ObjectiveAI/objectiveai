//! Making a tool: from a template, or joined to somebody else's
//! container.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::Origin;
use crate::store::{AccountId, Error, ToolId, counters};

/// What a new tool is made of: its origin, checked by the caller —
/// the template live, the provider on record — and what a create
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
        Origin::Connected { provider, id, .. } => {
            format!("tools:{}:{id}", serde_json::to_string(provider).map_err(Error::Json)?)
        }
    };
    let index = counters::next(conn, &key).await?;
    let (kind, template, provider, connected_provider, connected_id, authorization) = match &new.origin {
        Origin::Created { template, provider } => (
            "created",
            Some(template.clone()),
            provider.as_ref().map(Json),
            None,
            None,
            None,
        ),
        Origin::Connected { provider, id, authorization } => (
            "connected",
            None,
            None,
            Some(Json(provider)),
            Some(id.clone()),
            Some(authorization.clone()),
        ),
    };
    let inserted = sqlx::query(
        "INSERT INTO diverge.tools (kind, template, provider, connected_provider, connected_id, authorization, index, name, \
         account, fuse_file_mounts, fuse_directory_mounts, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(kind)
    .bind(template)
    .bind(provider)
    .bind(connected_provider)
    .bind(connected_id)
    .bind(authorization)
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
