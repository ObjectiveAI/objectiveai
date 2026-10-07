//! Reading an agent off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::Agent;
use crate::store::{AccountId, AgentId, Error};

/// The columns every agent query selects.
pub(super) const SELECT: &str = "SELECT id, template, index, name, account, provider, fuse_file_mounts, fuse_directory_mounts, \
     deployer, last_provider, last_active, tags, created, creator FROM diverge.agents";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The agent a row of [`SELECT`] holds.
pub(super) fn agent(row: &PgRow) -> Result<Agent, Error> {
    let index: i64 = row.try_get("index")?;
    let account: Option<i64> = row.try_get("account")?;
    let provider: Option<Json<Provider>> = row.try_get("provider")?;
    let Json(fuse_file_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_file_mounts")?;
    let Json(fuse_directory_mounts): Json<Vec<FuseMount>> = row.try_get("fuse_directory_mounts")?;
    let deployer: Option<Json<creator::Agent>> = row.try_get("deployer")?;
    let last_provider: Option<Json<Identity>> = row.try_get("last_provider")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Agent {
        id: AgentId(row.try_get("id")?),
        template: row.try_get("template")?,
        index: u64::try_from(index).unwrap_or(0),
        name: row.try_get("name")?,
        account: account.map(AccountId),
        provider: provider.map(|Json(provider)| provider),
        fuse_file_mounts,
        fuse_directory_mounts,
        deployer: deployer.map(|Json(deployer)| deployer),
        last_provider: last_provider.map(|Json(identity)| identity),
        last_active: row.try_get("last_active")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
