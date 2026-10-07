//! Reading roles off rows.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::grant::Grant;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::{Holder, Role};
use crate::store::{AccountId, Error, RoleId};

/// The columns every role query selects.
pub(super) const SELECT: &str = "SELECT id, name, description, grants, tags, created, creator FROM diverge.roles";

/// The order: oldest first.
pub(super) const ORDER: &str = "ORDER BY created, id";

/// The roles the rows of [`SELECT`] hold, with their holders loaded
/// in one more query.
pub(super) async fn roles(conn: &mut PgConnection, rows: &[PgRow]) -> Result<Vec<Role>, Error> {
    let mut roles = rows.iter().map(role).collect::<Result<Vec<Role>, Error>>()?;
    let ids: Vec<i64> = roles.iter().map(|role| role.id.0).collect();
    let held = sqlx::query(
        "SELECT ar.role, a.id, a.name, a.identity FROM diverge.account_roles ar \
         JOIN diverge.accounts a ON a.id = ar.account WHERE ar.role = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut holders: HashMap<i64, Vec<Holder>> = HashMap::new();
    for row in &held {
        let role: i64 = row.try_get("role")?;
        holders.entry(role).or_default().push(Holder {
            id: AccountId(row.try_get("id")?),
            name: row.try_get("name")?,
            identity: row.try_get("identity")?,
        });
    }
    for role in &mut roles {
        role.holders = holders.remove(&role.id.0).unwrap_or_default();
    }
    Ok(roles)
}

/// The role a row holds, with no holders yet.
fn role(row: &PgRow) -> Result<Role, Error> {
    let Json(grants): Json<Vec<Grant>> = row.try_get("grants")?;
    let Json(creator): Json<Creator> = row.try_get("creator")?;
    let created: DateTime<Utc> = row.try_get("created")?;
    Ok(Role {
        id: RoleId(row.try_get("id")?),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        grants,
        holders: Vec::new(),
        tags: row.try_get("tags")?,
        created,
        creator,
    })
}
