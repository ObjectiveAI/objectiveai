//! Making an account.

use std::net::IpAddr;

use diverge_sdk::daemon::creator::Creator;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{AccountId, Error};

/// What a new account is made of. The credential is three columns
/// together: an identity, the hash of a key the caller minted, and
/// maybe an address; all three absent is an account with no
/// credential.
#[derive(Debug, Clone)]
pub struct New {
    /// The name, if any.
    pub name: Option<String>,
    /// The credential's identity, if any.
    pub identity: Option<String>,
    /// The credential's address, if any.
    pub address: Option<IpAddr>,
    /// The SHA-256 of the credential's key, hex, if any.
    pub key_hash: Option<String>,
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
    /// The name or the identity is another account's already; nothing
    /// was made.
    Exists,
}

/// Insert the account, holding no role and no tag yet.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query(
        "INSERT INTO diverge.accounts (name, identity, address, key_hash, description, creator) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(&new.name)
    .bind(&new.identity)
    .bind(new.address.map(|address| address.to_string()))
    .bind(&new.key_hash)
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
