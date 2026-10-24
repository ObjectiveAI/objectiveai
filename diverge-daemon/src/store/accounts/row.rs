//! Reading an account off a row.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::Row as _;

use diverge_sdk::shared::Identity;

use super::Account;
use crate::store::{AccountId, Error};

/// The columns every account query selects, with the roles folded
/// in: one row per account.
pub(super) const SELECT: &str = "SELECT a.id, a.issuer, a.subject, a.description, a.tags, a.created, a.creator, \
     COALESCE(array_agg(r.name ORDER BY r.name) FILTER (WHERE r.id IS NOT NULL), '{}') AS roles \
     FROM diverge.accounts a \
     LEFT JOIN diverge.account_roles ar ON ar.account = a.id \
     LEFT JOIN diverge.roles r ON r.id = ar.role";

/// What follows the `WHERE`, if any: one row per account, oldest
/// first.
pub(super) const GROUP: &str = "GROUP BY a.id ORDER BY a.created, a.id";

/// The account a row of [`SELECT`] holds.
pub(super) fn account(row: &PgRow) -> Result<Account, Error> {
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Account {
        id: AccountId(row.try_get("id")?),
        identity: Identity { issuer: row.try_get("issuer")?, subject: row.try_get("subject")? },
        description: row.try_get("description")?,
        roles: row.try_get("roles")?,
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
