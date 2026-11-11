//! Reading a tool off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::agent_log::Identity;
use indexmap::IndexMap;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Tool;
use crate::store::{Error, ToolId};
use diverge_sdk::shared::permission::Grant;

/// The columns every tool query selects.
pub(super) const SELECT: &str = "SELECT id, template, provider, index, name, \
     fuse_file_mounts, fuse_directory_mounts, secrets, permissions, last_provider, last_deployed, last_active, tags, created, creator \
     FROM diverge.tools";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The tool a row of [`SELECT`] holds.
pub(super) fn tool(row: &PgRow) -> Result<Tool, Error> {
    let provider: Option<Json<Provider>> = row.try_get("provider")?;
    let index: i64 = row.try_get("index")?;
    let Json(fuse_file_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_file_mounts")?;
    let Json(fuse_directory_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_directory_mounts")?;
    let Json(secrets): Json<IndexMap<String, String>> = row.try_get("secrets")?;
    let Json(permissions): Json<Vec<Grant>> = row.try_get("permissions")?;
    let last_provider: Option<Json<Identity>> = row.try_get("last_provider")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Tool {
        id: ToolId(row.try_get("id")?),
        template: row.try_get("template")?,
        provider: provider.map(|Json(provider)| provider),
        index: u64::try_from(index).unwrap_or(0),
        name: row.try_get("name")?,
        fuse_file_mounts,
        fuse_directory_mounts,
        secrets,
        permissions,
        last_provider: last_provider.map(|Json(identity)| identity),
        last_deployed: row.try_get("last_deployed")?,
        last_active: row.try_get("last_active")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
