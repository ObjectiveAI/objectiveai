//! Adding an incoming credential.

use std::net::IpAddr;

use diverge_sdk::daemon::creator::Creator;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, IncomingId};

/// What a new credential is made of: the identity, the address if
/// any, and the hash of the key the caller minted.
#[derive(Debug, Clone)]
pub struct New {
    /// The identity a provider presenting the key has.
    pub identity: String,
    /// The one address the key is accepted from, if any.
    pub address: Option<IpAddr>,
    /// The SHA-256 of the key, hex.
    pub key_hash: String,
    /// Who added it.
    pub creator: Creator,
}

/// What an add came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The credential, added.
    Created(IncomingId),
    /// A credential names the identity already; nothing was added.
    Exists,
}

/// Insert the credential.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query(
        "INSERT INTO diverge.providers_incoming (identity, address, key_hash, creator) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&new.identity)
    .bind(new.address.map(|address| address.to_string()))
    .bind(&new.key_hash)
    .bind(Json(&new.creator))
    .fetch_one(&mut *conn)
    .await;
    match inserted {
        Ok(row) => Ok(Created::Created(IncomingId(row.try_get("id")?))),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
