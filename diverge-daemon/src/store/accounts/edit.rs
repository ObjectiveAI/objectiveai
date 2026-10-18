//! Changing an account's own columns.

use std::net::IpAddr;

use sqlx::PgConnection;

use crate::store::{AccountId, Error};

/// An account's own columns, as they are to be: the whole of them,
/// since an edit is applied whole. The caller computes them from the
/// account as loaded and the changes asked for.
#[derive(Debug, Clone)]
pub struct Columns {
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
}

/// What an update came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updated {
    /// The columns are as given.
    Updated,
    /// The name or the identity is another account's already; nothing
    /// changed.
    InUse,
}

/// Write the columns.
pub async fn update(conn: &mut PgConnection, id: AccountId, columns: &Columns) -> Result<Updated, Error> {
    let written = sqlx::query(
        "UPDATE diverge.accounts SET name = $2, identity = $3, address = $4, key_hash = $5, description = $6 WHERE id = $1",
    )
    .bind(id.0)
    .bind(&columns.name)
    .bind(&columns.identity)
    .bind(columns.address.map(|address| address.to_string()))
    .bind(&columns.key_hash)
    .bind(&columns.description)
    .execute(&mut *conn)
    .await;
    match written {
        Ok(_) => Ok(Updated::Updated),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Updated::InUse),
        Err(error) => Err(error.into()),
    }
}
