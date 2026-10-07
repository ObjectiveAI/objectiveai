//! Replacing an incoming credential.

use std::net::IpAddr;

use sqlx::PgConnection;

use crate::store::{Error, IncomingId};

/// A credential's columns, as they are to be: the whole of it, with
/// the hash of a key the caller minted anew.
#[derive(Debug, Clone)]
pub struct Columns {
    /// The identity a provider presenting the new key has.
    pub identity: String,
    /// The one address the new key is accepted from, if any.
    pub address: Option<IpAddr>,
    /// The SHA-256 of the new key, hex.
    pub key_hash: String,
}

/// What an update came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updated {
    /// The credential is as given, and the old key admits nothing.
    Updated,
    /// The identity is another credential's already; nothing changed.
    InUse,
}

/// Write the columns.
pub async fn update(conn: &mut PgConnection, id: IncomingId, columns: &Columns) -> Result<Updated, Error> {
    let written = sqlx::query("UPDATE diverge.providers_incoming SET identity = $2, address = $3, key_hash = $4 WHERE id = $1")
        .bind(id.0)
        .bind(&columns.identity)
        .bind(columns.address.map(|address| address.to_string()))
        .bind(&columns.key_hash)
        .execute(&mut *conn)
        .await;
    match written {
        Ok(_) => Ok(Updated::Updated),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Updated::InUse),
        Err(error) => Err(error.into()),
    }
}
