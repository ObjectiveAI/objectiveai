//! Reading a tool off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::{Origin, Tool};
use crate::store::{AccountId, Error, ToolId};

/// The columns every tool query selects.
pub(super) const SELECT: &str = "SELECT id, kind, template, provider, connected_provider, connected_id, authorization, index, name, \
     account, fuse_file_mounts, fuse_directory_mounts, last_provider, last_active, tags, created, creator \
     FROM diverge.tools";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The tool a row of [`SELECT`] holds.
pub(super) fn tool(row: &PgRow) -> Result<Tool, Error> {
    let kind: String = row.try_get("kind")?;
    let origin = match kind.as_str() {
        "created" => {
            let provider: Option<Json<Provider>> = row.try_get("provider")?;
            Origin::Created {
                template: row.try_get::<Option<String>, _>("template")?.unwrap_or_default(),
                provider: provider.map(|Json(provider)| provider),
            }
        }
        "connected" => {
            let provider: Option<Json<Identity>> = row.try_get("connected_provider")?;
            let Some(Json(provider)) = provider else {
                return Err(Error::Json(serde::de::Error::custom("a connected tool names no provider")));
            };
            Origin::Connected {
                provider,
                id: row.try_get::<Option<String>, _>("connected_id")?.unwrap_or_default(),
                authorization: row.try_get::<Option<String>, _>("authorization")?.unwrap_or_default(),
            }
        }
        other => {
            return Err(Error::Json(serde::de::Error::custom(format!("`{other}` is not a tool kind"))));
        }
    };
    let index: i64 = row.try_get("index")?;
    let account: Option<i64> = row.try_get("account")?;
    let Json(fuse_file_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_file_mounts")?;
    let Json(fuse_directory_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_directory_mounts")?;
    let last_provider: Option<Json<Identity>> = row.try_get("last_provider")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Tool {
        id: ToolId(row.try_get("id")?),
        origin,
        index: u64::try_from(index).unwrap_or(0),
        name: row.try_get("name")?,
        account: account.map(AccountId),
        fuse_file_mounts,
        fuse_directory_mounts,
        last_provider: last_provider.map(|Json(identity)| identity),
        last_active: row.try_get("last_active")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
