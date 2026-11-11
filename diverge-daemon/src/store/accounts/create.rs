//! Making an account.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::Identity;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{AccountId, Error};

/// What a new account is made of: the identity it is, what it is
/// for, and who made it.
#[derive(Debug, Clone)]
pub struct New {
    /// The identity the account is.
    pub identity: Identity,
    /// What the account is for.
    pub description: Option<String>,
    /// Who made it.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The account, made.
    Created(AccountId),
    /// The identity is another account's already; nothing was made.
    Exists,
}

/// Insert the account, holding no role and no tag yet.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query(
        "INSERT INTO diverge.accounts (issuer, subject, description, creator) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&new.identity.issuer)
    .bind(&new.identity.subject)
    .bind(&new.description)
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(AccountId(row.try_get("id")?))),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
