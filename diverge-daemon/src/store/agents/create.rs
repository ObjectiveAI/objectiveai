//! Making an agent.

use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{AccountId, AgentId, Error, counters};

/// What a new agent is made of: the create's members, checked by the
/// caller — the template live, the account found and assignable, the
/// provider on record, the deployer the caller's.
#[derive(Debug, Clone)]
pub struct New {
    /// The template, by id.
    pub template: String,
    /// The name, if any.
    pub name: Option<String>,
    /// The account it runs under, if any.
    pub account: Option<AccountId>,
    /// The provider pin and its volumes, if any.
    pub provider: Option<Provider>,
    /// Files of other providers' volumes.
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of other providers' volumes.
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The deployer agent as it is now, if any.
    pub deployer: Option<creator::Agent>,
    /// Who makes it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The agent, made, with the index it was given.
    Created(AgentId, u64),
    /// The name is another agent's already; nothing was made.
    Exists,
}

/// Insert the agent, its index the next for its template, holding no
/// tag yet. The counter is advanced in the caller's transaction, so
/// a create that is not committed gives the number back.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let index = counters::next(conn, &format!("agents:{}", new.template)).await?;
    let inserted = sqlx::query(
        "INSERT INTO diverge.agents (template, index, name, account, provider, fuse_file_mounts, fuse_directory_mounts, deployer, creator) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(&new.template)
    .bind(i64::try_from(index).unwrap_or(i64::MAX))
    .bind(&new.name)
    .bind(new.account.map(|account| account.0))
    .bind(new.provider.as_ref().map(Json))
    .bind(Json(&new.fuse_file_mounts))
    .bind(Json(&new.fuse_directory_mounts))
    .bind(new.deployer.as_ref().map(Json))
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(AgentId(row.try_get("id")?), index)),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
