//! Making a tool from a template.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use indexmap::IndexMap;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, ToolId, counters, tool_names};

/// What a new tool is made of, checked by the caller — the template
/// live, the provider and the mounts there — and what its create adds.
#[derive(Debug, Clone)]
pub struct New {
    /// The template, by id.
    pub template: String,
    /// The provider pin and its volumes, if any.
    pub provider: Option<Provider>,
    /// The name, if any.
    pub name: Option<String>,
    /// Files of other providers' volumes.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// Which vault key each secret the template declares is.
    pub secrets: IndexMap<String, String>,
    /// Who makes it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The tool, made, with the index it was given.
    Created(ToolId, u64),
    /// The name is a standalone or a connected tool's already; nothing
    /// was made.
    Exists,
}

/// Insert the tool, its index the next among tools made from its
/// template, holding no tag yet, its name claimed across both tool
/// families as [`tool_names`] states. The counter is advanced in the
/// caller's transaction, so a create that is not committed gives the
/// number back.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    if tool_names::claim(conn, new.name.as_deref(), None).await? {
        return Ok(Created::Exists);
    }
    let index = counters::next(conn, &format!("tools:{}", new.template)).await?;
    let inserted = sqlx::query(
        "INSERT INTO diverge.tools (template, provider, index, name, \
         fuse_file_mounts, fuse_directory_mounts, secrets, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(&new.template)
    .bind(new.provider.as_ref().map(Json))
    .bind(i64::try_from(index).unwrap_or(i64::MAX))
    .bind(&new.name)
    .bind(Json(&new.fuse_file_mounts))
    .bind(Json(&new.fuse_directory_mounts))
    .bind(Json(&new.secrets))
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(ToolId(row.try_get("id")?), index)),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
