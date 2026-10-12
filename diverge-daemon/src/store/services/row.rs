//! Reading a service off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::shared::containers::dependencies::Template;
use sqlx::Row as _;
use sqlx::postgres::PgRow;
use sqlx::types::Json;

use super::{Service, scope_of};
use crate::store::{AgentId, Error, ServiceId};

/// The columns every service query selects.
pub(super) const SELECT: &str = "SELECT id, template, scope, owner_agent, owner_template, declared, last_provider, created, creator \
     FROM diverge.services";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The service a row of [`SELECT`] holds.
pub(super) fn service(row: &PgRow) -> Result<Service, Error> {
    let scope: String = row.try_get("scope")?;
    let Some(scope) = scope_of(&scope) else {
        return Err(Error::Json(serde::de::Error::custom(format!("`{scope}` is not a service scope"))));
    };
    let owner_agent: Option<i64> = row.try_get("owner_agent")?;
    let Json(declared): Json<Template> = row.try_get("declared")?;
    let last_provider: Option<Json<Identity>> = row.try_get("last_provider")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Service {
        id: ServiceId(row.try_get("id")?),
        template: row.try_get("template")?,
        scope,
        owner_agent: owner_agent.map(AgentId),
        owner_template: row.try_get("owner_template")?,
        declared,
        last_provider: last_provider.map(|Json(identity)| identity),
        created,
        creator,
    })
}
