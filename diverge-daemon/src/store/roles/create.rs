//! Making a role.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::grant::Grant;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, RoleId};

/// What a new role is made of.
#[derive(Debug, Clone)]
pub struct New {
    /// The name.
    pub name: String,
    /// What the role is for.
    pub description: Option<String>,
    /// Its grants, as given.
    pub grants: Vec<Grant>,
    /// Who made it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The role, made.
    Created(RoleId),
    /// The name is another role's already; nothing was made.
    Exists,
}

/// Insert the role, held by nobody and with no tag yet.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query(
        "INSERT INTO diverge.roles (name, description, grants, creator) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&new.name)
    .bind(&new.description)
    .bind(Json(&new.grants))
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(RoleId(row.try_get("id")?))),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
